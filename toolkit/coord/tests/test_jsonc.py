from __future__ import annotations

import json

from hypothesis import example, given
from hypothesis import strategies as st

from ai_coord.jsonc import JsoncDocument

JSON_SCALARS = st.none() | st.booleans() | st.integers() | st.text()


@example(value="https://example.test/a//b/*c*/", items=[])
@given(value=JSON_SCALARS, items=st.lists(JSON_SCALARS, max_size=5))
def test_parse_serialize_without_mutations_is_byte_identical(
    value: object, items: list[object]
) -> None:
    rendered_items = ",\n".join(f"    {json.dumps(item)}" for item in items)
    separator = ",\n" if rendered_items else ""
    text = f"""{{
  // header
  "value": {json.dumps(value)},
  "items": [
{rendered_items}{separator}  ],
  /* footer */
}}
"""

    document = JsoncDocument.parse(text)

    assert document.value == {"value": value, "items": items}
    assert document.text == text
