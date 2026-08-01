"""
Tests for Codex CLI notify handler.
"""

from ai_notify.config_loader import AINotifyConfig, NotificationConfig, NotificationMode
from ai_notify.events.codex import handle_codex_notify


def test_codex_notify_sends_notification(mocker):
    config = AINotifyConfig(notification=NotificationConfig())
    mocker.patch("ai_notify.events.codex.get_runtime_config", return_value=config)
    mocker.patch("ai_notify.events.codex.os.getcwd", return_value="/Users/test/project")

    mock_notifier = mocker.patch("ai_notify.events.codex.MacNotifier")
    mock_notifier.get_project_name.return_value = "project"
    notifier_instance = mock_notifier.return_value

    payload = {
        "type": "agent-turn-complete",
        "input-messages": ["Fix the bug"],
        "last-assistant-message": "Done.",
    }

    handle_codex_notify(payload)

    mock_notifier.assert_called_once_with(icon_name="codex")
    notifier_instance.notify_completion.assert_called_once_with(
        "project",
        agent="Codex",
        task="Fix the bug",
        result="Done.",
    )


def test_codex_notify_respects_disabled_mode(mocker):
    config = AINotifyConfig(notification=NotificationConfig(mode=NotificationMode.DISABLED))
    mocker.patch("ai_notify.events.codex.get_runtime_config", return_value=config)
    mocker.patch("ai_notify.events.codex.os.getcwd", return_value="/Users/test/project")

    mock_notifier = mocker.patch("ai_notify.events.codex.MacNotifier")

    payload = {
        "type": "agent-turn-complete",
        "input-messages": ["Fix the bug"],
        "last-assistant-message": "Done.",
    }

    handle_codex_notify(payload)

    assert not mock_notifier.return_value.notify_completion.called


def test_codex_notify_respects_exclude_patterns(mocker):
    config = AINotifyConfig(
        notification=NotificationConfig(exclude_patterns=["/skip"], mode=NotificationMode.ALL)
    )
    mocker.patch("ai_notify.events.codex.get_runtime_config", return_value=config)
    mocker.patch("ai_notify.events.codex.os.getcwd", return_value="/Users/test/project")

    mock_notifier = mocker.patch("ai_notify.events.codex.MacNotifier")

    payload = {
        "type": "agent-turn-complete",
        "input-messages": ["/skip build"],
        "last-assistant-message": "Done.",
    }

    handle_codex_notify(payload)

    assert not mock_notifier.return_value.notify_completion.called


def test_codex_notify_does_not_duplicate_prompt_when_result_is_missing(mocker):
    config = AINotifyConfig(notification=NotificationConfig())
    mocker.patch("ai_notify.events.codex.get_runtime_config", return_value=config)
    mocker.patch("ai_notify.events.codex.os.getcwd", return_value="/Users/test/project")
    mock_notifier = mocker.patch("ai_notify.events.codex.MacNotifier")
    mock_notifier.get_project_name.return_value = "project"

    handle_codex_notify(
        {
            "type": "agent-turn-complete",
            "input-messages": ["Fix the bug"],
        }
    )

    mock_notifier.return_value.notify_completion.assert_called_once_with(
        "project",
        agent="Codex",
        task="Fix the bug",
        result="",
    )
