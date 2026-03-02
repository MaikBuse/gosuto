# Unit Test Plan

## Current Coverage

| File | Tests | Description |
|------|-------|-------------|
| `src/config.rs` | 4 | Store path extraction & sanitization |
| `src/matrix/client.rs` | 8 | `normalize_homeserver_url` edge cases |
| `src/ui/login.rs` | 9 | Login form field navigation & validation |
| **Total** | **21** | |

## Test Priority Tiers

### Tier 1 — Pure State Logic (highest value, zero dependencies)

These modules contain pure functions and state machines with no I/O, async, or external dependencies. Every method is deterministic and directly testable.

| Module | Key Areas | Est. Tests |
|--------|-----------|------------|
| `src/state/messages.rs` | Dedup (event_id), prepend ordering, confirm_sent, scroll bounds, set_room reset | ~12 |
| `src/state/members.rs` | Sort order (power + name), navigation bounds, selected_member, clear | ~10 |
| `src/state/rooms.rs` | Display row building (spaces, orphans, DMs), nav skipping, toggle, filter, domain extraction | ~18 |
| `src/state/auth.rs` | `is_logged_in` per variant | ~6 |

### Tier 2 — Input Handling (pure functions, crossterm types)

Event handlers are pure `(KeyEvent, &mut VimState) -> InputResult` functions. Only dependency is `crossterm::event` types for constructing test inputs.

| Module | Key Areas | Est. Tests |
|--------|-----------|------------|
| `src/input/vim.rs` | CompletionState cycling, VimState mode transitions, text buffer ops | ~18 |
| `src/input/normal.rs` | Key→action mappings, `gg` sequence, search mode flow | ~14 |
| `src/input/insert.rs` | Esc/Enter/Char/Backspace behavior | ~6 |
| `src/input/command.rs` | `filtered_commands`, `parse_command`, tab completion, key handling | ~16 |

### Tier 3 — Utilities & Effects

| Module | Key Areas | Est. Tests |
|--------|-----------|------------|
| `src/ui/theme.rs` | `sender_color` determinism & distribution | ~4 |
| `src/ui/effects/mod.rs` | Xorshift64 PRNG, EffectsState toggles | ~10 |
| `src/voip/state.rs` | CallInfo factories, elapsed_display formatting | ~6 |
| `src/config.rs` | Default values, serialization roundtrip (extend existing) | ~4 |

### Tier 4 — Integration-heavy (deferred)

These modules require async runtimes, Matrix SDK clients, or TUI rendering pipelines. They are not suitable for unit tests without significant mocking infrastructure.

| Module | Reason |
|--------|--------|
| `src/matrix/client.rs` | Async + Matrix SDK client (already well-tested for pure helper) |
| `src/matrix/sync.rs` | Async event handlers |
| `src/app.rs` | Large aggregate struct, requires EventSender channel |
| `src/ui/*.rs` (render) | Requires ratatui Frame + terminal setup |

## Specific Test Cases

### `src/state/messages.rs` — MessageState

```
add_message_with_unique_event_id
add_message_dedup_same_event_id
add_message_empty_event_id_always_added
prepend_messages_filters_duplicates
prepend_messages_ordering
prepend_messages_sets_has_more
confirm_sent_matches_pending_by_body
confirm_sent_no_match_leaves_unchanged
set_room_clears_state
set_room_noop_on_same_room
scroll_up_increments
scroll_down_decrements
scroll_down_saturates_at_zero
scroll_to_bottom_resets
```

### `src/state/members.rs` — MemberListState

```
set_members_sorts_by_power_desc_then_name_asc
set_members_case_insensitive_sort
move_up_decrements
move_up_saturates_at_zero
move_down_increments
move_down_clamps_to_last
move_down_empty_list
move_top_sets_zero
move_bottom_sets_last
selected_member_returns_correct
selected_member_empty_list
clear_resets_all
```

### `src/state/rooms.rs` — RoomListState

```
extract_server_domain_standard
extract_server_domain_no_colon
rebuild_with_spaces_and_children
rebuild_orphan_rooms_grouped_by_domain
rebuild_dms_section
rebuild_search_filter
rebuild_collapsed_space_hides_children
move_up_skips_section_headers
move_down_skips_call_participants
move_top_finds_first_navigable
move_bottom_finds_last_navigable
toggle_space_collapse_expand
set_filter_rebuilds
selected_room_on_room_row
selected_room_on_header_returns_none
empty_rooms_list
```

### `src/state/auth.rs` — AuthState

```
logged_out_is_not_logged_in
logging_in_is_not_logged_in
auto_logging_in_is_not_logged_in
registering_is_not_logged_in
logged_in_is_logged_in
error_is_not_logged_in
```

### `src/input/vim.rs` — VimState + CompletionState

```
completion_next_from_none
completion_next_wraps
completion_prev_from_none
completion_prev_wraps
completion_next_empty_match_count
completion_reset
vim_new_defaults
enter_insert_sets_mode
enter_normal_clears_state
enter_command_clears_buffer
enter_command_with_prefills
insert_char_ascii
insert_char_multibyte
backspace_removes_char
backspace_at_start_noop
take_input_returns_and_clears
clear_input
```

### `src/input/normal.rs` — handle_normal

```
j_moves_down
k_moves_up
G_moves_bottom
gg_moves_top
q_quits
ctrl_c_quits
tab_switches_panel
h_focus_left
l_focus_right
enter_selects
i_enters_insert
colon_enters_command
slash_enters_search
search_char_appends
search_backspace_pops
search_enter_confirms
search_esc_cancels
space_shows_which_key
```

### `src/input/insert.rs` — handle_insert

```
esc_returns_to_normal
enter_sends_message
enter_empty_returns_none
char_inserts
backspace_delegates
ctrl_c_quits
```

### `src/input/command.rs` — handle_command + parse_command

```
filtered_commands_empty_returns_all
filtered_commands_partial_filters
filtered_commands_alias_matches
parse_quit
parse_q_alias
parse_join_with_arg
parse_join_no_arg_returns_none
parse_leave
parse_dm
parse_all_aliases
parse_verify_no_arg
parse_verify_with_arg
parse_unknown_returns_none
esc_exits_command_mode
backspace_last_char_exits
char_appends_to_buffer
```

### `src/ui/theme.rs`

```
sender_color_deterministic
sender_color_different_inputs
sender_color_empty_string
```

### `src/ui/effects/mod.rs` — Xorshift64 + EffectsState

```
xorshift_zero_seed_uses_fallback
xorshift_deterministic_sequence
xorshift_next_f32_in_range
xorshift_next_range_bounds
xorshift_next_u32_range_min_eq_max
xorshift_next_u32_range_values_in_range
effects_toggle_flips
effects_toggle_glitch_flips
effects_initial_state
```

### `src/voip/state.rs` — CallInfo

```
new_outgoing_fields
new_incoming_fields
elapsed_display_no_start
elapsed_display_with_start
```

### `src/config.rs` (extend existing)

```
default_config_values
config_roundtrip_toml
effects_default_enabled
audio_default_values
```

## Testing Patterns

- All tests use standard `#[cfg(test)] mod tests` with `use super::*`
- No dev-dependencies required — only `crossterm::event` types (already a dependency)
- Helper functions within test modules for constructing common test fixtures (e.g., `make_msg()`, `make_key()`)
- Use `matches!()` macro for `InputResult` / `CommandAction` assertions (no `PartialEq` derive)
- Boundary testing: empty collections, zero/max indices, UTF-8 multibyte characters

## Estimated Total

~124 new test functions across 13 files.
