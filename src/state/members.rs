#[derive(Debug, Clone)]
pub struct RoomMember {
    pub user_id: String,
    pub display_name: String,
    pub power_level: i64,
}

#[derive(Debug)]
pub struct MemberListState {
    pub members: Vec<RoomMember>,
    pub selected: usize,
    pub current_room_id: Option<String>,
}

impl MemberListState {
    pub fn new() -> Self {
        Self {
            members: Vec::new(),
            selected: 0,
            current_room_id: None,
        }
    }

    pub fn set_members(&mut self, room_id: &str, mut members: Vec<RoomMember>) {
        // Sort: highest power level first, then alphabetical
        members.sort_by(|a, b| {
            b.power_level.cmp(&a.power_level).then(
                a.display_name
                    .to_lowercase()
                    .cmp(&b.display_name.to_lowercase()),
            )
        });
        self.current_room_id = Some(room_id.to_string());
        self.members = members;
        self.selected = 0;
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        if !self.members.is_empty() {
            self.selected = (self.selected + 1).min(self.members.len().saturating_sub(1));
        }
    }

    pub fn move_top(&mut self) {
        self.selected = 0;
    }

    pub fn move_bottom(&mut self) {
        if !self.members.is_empty() {
            self.selected = self.members.len().saturating_sub(1);
        }
    }

    pub fn selected_member(&self) -> Option<&RoomMember> {
        self.members.get(self.selected)
    }

    pub fn clear(&mut self) {
        self.members.clear();
        self.selected = 0;
        self.current_room_id = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(user_id: &str, name: &str, power: i64) -> RoomMember {
        RoomMember {
            user_id: user_id.to_string(),
            display_name: name.to_string(),
            power_level: power,
        }
    }

    #[test]
    fn set_members_sorts_by_power_desc_then_name_asc() {
        let mut state = MemberListState::new();
        state.set_members(
            "!room:x",
            vec![
                member("@c:x", "Charlie", 0),
                member("@a:x", "Alice", 100),
                member("@b:x", "Bob", 50),
            ],
        );
        assert_eq!(state.members[0].display_name, "Alice");
        assert_eq!(state.members[1].display_name, "Bob");
        assert_eq!(state.members[2].display_name, "Charlie");
    }

    #[test]
    fn set_members_case_insensitive_sort() {
        let mut state = MemberListState::new();
        state.set_members(
            "!room:x",
            vec![member("@b:x", "bob", 0), member("@a:x", "Alice", 0)],
        );
        assert_eq!(state.members[0].display_name, "Alice");
        assert_eq!(state.members[1].display_name, "bob");
    }

    #[test]
    fn set_members_resets_selection() {
        let mut state = MemberListState::new();
        state.set_members(
            "!room:x",
            vec![member("@a:x", "A", 0), member("@b:x", "B", 0)],
        );
        state.selected = 1;
        state.set_members("!room:x", vec![member("@c:x", "C", 0)]);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn move_up_decrements() {
        let mut state = MemberListState::new();
        state.set_members("!r:x", vec![member("@a:x", "A", 0), member("@b:x", "B", 0)]);
        state.selected = 1;
        state.move_up();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn move_up_saturates_at_zero() {
        let mut state = MemberListState::new();
        state.set_members("!r:x", vec![member("@a:x", "A", 0)]);
        state.move_up();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn move_down_increments() {
        let mut state = MemberListState::new();
        state.set_members(
            "!r:x",
            vec![
                member("@a:x", "A", 0),
                member("@b:x", "B", 0),
                member("@c:x", "C", 0),
            ],
        );
        state.move_down();
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn move_down_clamps_to_last() {
        let mut state = MemberListState::new();
        state.set_members("!r:x", vec![member("@a:x", "A", 0), member("@b:x", "B", 0)]);
        state.selected = 1;
        state.move_down();
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn move_down_empty_list() {
        let mut state = MemberListState::new();
        state.move_down();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn move_top_sets_zero() {
        let mut state = MemberListState::new();
        state.set_members("!r:x", vec![member("@a:x", "A", 0), member("@b:x", "B", 0)]);
        state.selected = 1;
        state.move_top();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn move_bottom_sets_last() {
        let mut state = MemberListState::new();
        state.set_members("!r:x", vec![member("@a:x", "A", 0), member("@b:x", "B", 0)]);
        state.move_bottom();
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn move_bottom_empty_list() {
        let mut state = MemberListState::new();
        state.move_bottom();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn selected_member_returns_correct() {
        let mut state = MemberListState::new();
        state.set_members(
            "!r:x",
            vec![member("@a:x", "Alice", 0), member("@b:x", "Bob", 0)],
        );
        state.selected = 1;
        let m = state.selected_member().unwrap();
        assert_eq!(m.display_name, "Bob");
    }

    #[test]
    fn selected_member_empty_list() {
        let state = MemberListState::new();
        assert!(state.selected_member().is_none());
    }

    #[test]
    fn clear_resets_all() {
        let mut state = MemberListState::new();
        state.set_members("!r:x", vec![member("@a:x", "A", 0)]);
        state.selected = 0;
        state.clear();
        assert!(state.members.is_empty());
        assert_eq!(state.selected, 0);
        assert!(state.current_room_id.is_none());
    }
}
