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
            b.power_level
                .cmp(&a.power_level)
                .then(a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()))
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
