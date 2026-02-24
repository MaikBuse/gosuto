#[derive(Debug, Clone)]
pub struct RoomMember {
    pub user_id: String,
    pub display_name: String,
    pub power_level: i64,
}

#[derive(Debug)]
pub struct MemberListState {
    pub members: Vec<RoomMember>,
    pub scroll_offset: usize,
    pub current_room_id: Option<String>,
}

impl MemberListState {
    pub fn new() -> Self {
        Self {
            members: Vec::new(),
            scroll_offset: 0,
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
        self.scroll_offset = 0;
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        if !self.members.is_empty() {
            self.scroll_offset = (self.scroll_offset + 1).min(self.members.len().saturating_sub(1));
        }
    }

    pub fn scroll_top(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn scroll_bottom(&mut self) {
        if !self.members.is_empty() {
            self.scroll_offset = self.members.len().saturating_sub(1);
        }
    }

    pub fn clear(&mut self) {
        self.members.clear();
        self.scroll_offset = 0;
        self.current_room_id = None;
    }
}
