# MatrixRTC m.call.member Event Format

Reference documentation for the `m.call.member` state event as sent by
Element X (the canonical mobile implementation).

## Event structure

**State key:** `_@user:server_DEVICEID_m.call` (per MSC4143)

**Content (on join):**

```json
{
  "application": "m.call",
  "call_id": "",
  "scope": "m.room",
  "device_id": "MYDEVICE",
  "expires": 7200000,
  "focus_active": {
    "type": "livekit",
    "focus_selection": "oldest_membership"
  },
  "foci_preferred": [{
    "type": "livekit",
    "livekit_alias": "!roomid:example.com",
    "livekit_service_url": "https://livekit-jwt.example.com"
  }]
}
```

**Content (on leave):** `{}`

## Field details

| Field | Value | Notes |
|-------|-------|-------|
| `application` | `"m.call"` | Fixed; also appended to state key |
| `call_id` | `""` | Empty string for room-scoped calls |
| `scope` | `"m.room"` | Room-scoped |
| `device_id` | Device ID string | From Matrix client |
| `expires` | `7200000` | 2 hours in ms |
| `focus_active` | Object | Always `livekit` + `oldest_membership` |
| `foci_preferred` | Array | Transport config from well-known / MSC4143 |

## State key format (MSC4143)

The state key follows the pattern `_{user_id}_{device_id}_{application}`:

```
_@user:example.com_ABCDEF1234_m.call
```

This allows multiple applications to use MatrixRTC simultaneously without
conflicting state keys.

## Key differences from naive implementations

- State key must include `_m.call` suffix (MSC4143 format)
- `livekit_alias` must be the **room ID**, not an empty string
- `expires` should be 2 hours (7200000ms) to match Element X
- Do **not** include `membershipID` or `m.call.intent` — Element X does not send these
- `created_ts` should only be set when re-publishing an existing membership
