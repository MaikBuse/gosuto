#[derive(Debug, Clone)]
pub enum RoomCategory {
    Space,
    Room,
    DirectMessage,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RoomSummary {
    pub id: String,
    pub name: String,
    pub category: RoomCategory,
    pub unread_count: u64,
    pub is_space_child: bool,
    pub parent_space_id: Option<String>,
}

#[derive(Debug)]
pub struct RoomListState {
    pub rooms: Vec<RoomSummary>,
    pub filtered_indices: Vec<usize>,
    pub selected: usize,
    pub search_filter: Option<String>,
}

impl RoomListState {
    pub fn new() -> Self {
        Self {
            rooms: Vec::new(),
            filtered_indices: Vec::new(),
            selected: 0,
            search_filter: None,
        }
    }

    pub fn set_rooms(&mut self, rooms: Vec<RoomSummary>) {
        self.rooms = rooms;
        self.refilter();
        if self.selected >= self.visible_len() {
            self.selected = self.visible_len().saturating_sub(1);
        }
    }

    pub fn visible_len(&self) -> usize {
        self.filtered_indices.len()
    }

    pub fn selected_room(&self) -> Option<&RoomSummary> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&i| self.rooms.get(i))
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.visible_len() > 0 && self.selected < self.visible_len() - 1 {
            self.selected += 1;
        }
    }

    pub fn move_top(&mut self) {
        self.selected = 0;
    }

    pub fn move_bottom(&mut self) {
        if self.visible_len() > 0 {
            self.selected = self.visible_len() - 1;
        }
    }

    pub fn set_filter(&mut self, query: Option<String>) {
        self.search_filter = query;
        self.refilter();
        if self.selected >= self.visible_len() {
            self.selected = self.visible_len().saturating_sub(1);
        }
    }

    fn refilter(&mut self) {
        self.filtered_indices = match &self.search_filter {
            None => (0..self.rooms.len()).collect(),
            Some(q) => {
                let q = q.to_lowercase();
                self.rooms
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| r.name.to_lowercase().contains(&q))
                    .map(|(i, _)| i)
                    .collect()
            }
        };
    }

    pub fn visible_rooms(&self) -> Vec<(usize, &RoomSummary)> {
        self.filtered_indices
            .iter()
            .enumerate()
            .filter_map(|(vi, &ri)| self.rooms.get(ri).map(|r| (vi, r)))
            .collect()
    }
}
