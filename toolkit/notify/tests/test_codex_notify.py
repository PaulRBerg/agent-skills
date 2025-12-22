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

    assert notifier_instance.send_notification.called
    args = notifier_instance.send_notification.call_args.kwargs
    assert args["title"] == "project"
    assert "Codex turn complete" in args["subtitle"]
    assert "Done." in (args.get("message") or "")


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

    assert not mock_notifier.return_value.send_notification.called


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

    assert not mock_notifier.return_value.send_notification.called
