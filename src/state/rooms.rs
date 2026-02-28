use std::collections::{HashMap, HashSet};

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

#[derive(Debug, Clone)]
pub enum DisplayRow {
    SectionHeader {
        label: String,
    },
    SpaceHeader {
        space_id: String,
        name: String,
        collapsed: bool,
        unread_count: u64,
    },
    Room {
        room_index: usize,
        indent: u8,
    },
}

#[derive(Debug)]
pub struct RoomListState {
    pub rooms: Vec<RoomSummary>,
    pub display_rows: Vec<DisplayRow>,
    pub selected: usize,
    pub search_filter: Option<String>,
    collapsed_spaces: HashMap<String, bool>,
}

impl RoomListState {
    pub fn new() -> Self {
        Self {
            rooms: Vec::new(),
            display_rows: Vec::new(),
            selected: 0,
            search_filter: None,
            collapsed_spaces: HashMap::new(),
        }
    }

    pub fn set_rooms(&mut self, rooms: Vec<RoomSummary>) {
        self.rooms = rooms;
        self.rebuild_display_rows();
        if self.display_rows.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.display_rows.len() {
            self.selected = self.display_rows.len().saturating_sub(1);
            self.skip_to_navigable();
        }
    }

    pub fn rebuild_display_rows(&mut self) {
        let filter = self.search_filter.as_ref().map(|q| q.to_lowercase());
        let mut rows = Vec::new();

        // Collect space indices
        let spaces: Vec<usize> = self
            .rooms
            .iter()
            .enumerate()
            .filter(|(_, r)| matches!(r.category, RoomCategory::Space))
            .map(|(i, _)| i)
            .collect();

        // Build children map: space_id -> Vec<room_index> (DMs excluded)
        let mut space_children: HashMap<&str, Vec<usize>> = HashMap::new();
        for &si in &spaces {
            space_children.insert(&self.rooms[si].id, Vec::new());
        }

        let mut claimed_by_space: HashSet<usize> = HashSet::new();
        for (i, room) in self.rooms.iter().enumerate() {
            if matches!(room.category, RoomCategory::DirectMessage) {
                continue;
            }
            if let Some(ref parent) = room.parent_space_id
                && let Some(children) = space_children.get_mut(parent.as_str())
            {
                children.push(i);
                claimed_by_space.insert(i);
            }
        }

        // Emit space groups
        for &si in &spaces {
            let space = &self.rooms[si];
            let children = space_children
                .get(space.id.as_str())
                .cloned()
                .unwrap_or_default();

            // Filter children by search query
            let filtered_children: Vec<usize> = if let Some(ref q) = filter {
                children
                    .iter()
                    .copied()
                    .filter(|&ci| self.rooms[ci].name.to_lowercase().contains(q))
                    .collect()
            } else {
                children
            };

            // Skip space if filter active and no matching children
            if filter.is_some() && filtered_children.is_empty() {
                continue;
            }

            let collapsed =
                filter.is_none() && *self.collapsed_spaces.get(&space.id).unwrap_or(&false);

            // Aggregate unread for collapsed spaces
            let unread = if collapsed {
                filtered_children
                    .iter()
                    .map(|&ci| self.rooms[ci].unread_count)
                    .sum()
            } else {
                0
            };

            rows.push(DisplayRow::SpaceHeader {
                space_id: space.id.clone(),
                name: space.name.clone(),
                collapsed,
                unread_count: unread,
            });

            if !collapsed {
                for ci in filtered_children {
                    rows.push(DisplayRow::Room {
                        room_index: ci,
                        indent: 2,
                    });
                }
            }
        }

        // Orphan rooms: regular rooms not claimed by any space
        let orphans: Vec<usize> = self
            .rooms
            .iter()
            .enumerate()
            .filter(|(i, r)| {
                matches!(r.category, RoomCategory::Room) && !claimed_by_space.contains(i)
            })
            .filter(|(_, r)| {
                filter
                    .as_ref()
                    .is_none_or(|q| r.name.to_lowercase().contains(q))
            })
            .map(|(i, _)| i)
            .collect();

        if !orphans.is_empty() {
            rows.push(DisplayRow::SectionHeader {
                label: "ROOMS".to_string(),
            });
            for oi in orphans {
                rows.push(DisplayRow::Room {
                    room_index: oi,
                    indent: 0,
                });
            }
        }

        // DMs: always in their own section
        let dms: Vec<usize> = self
            .rooms
            .iter()
            .enumerate()
            .filter(|(_, r)| matches!(r.category, RoomCategory::DirectMessage))
            .filter(|(_, r)| {
                filter
                    .as_ref()
                    .is_none_or(|q| r.name.to_lowercase().contains(q))
            })
            .map(|(i, _)| i)
            .collect();

        if !dms.is_empty() {
            rows.push(DisplayRow::SectionHeader {
                label: "DIRECT MESSAGES".to_string(),
            });
            for di in dms {
                rows.push(DisplayRow::Room {
                    room_index: di,
                    indent: 0,
                });
            }
        }

        self.display_rows = rows;
    }

    fn is_navigable(&self, index: usize) -> bool {
        matches!(
            self.display_rows.get(index),
            Some(DisplayRow::SpaceHeader { .. } | DisplayRow::Room { .. })
        )
    }

    fn skip_to_navigable(&mut self) {
        // Try forward from current position
        for i in self.selected..self.display_rows.len() {
            if self.is_navigable(i) {
                self.selected = i;
                return;
            }
        }
        // Try backward
        for i in (0..self.selected).rev() {
            if self.is_navigable(i) {
                self.selected = i;
                return;
            }
        }
        self.selected = 0;
    }

    pub fn move_up(&mut self) {
        if self.display_rows.is_empty() {
            return;
        }
        let mut pos = self.selected;
        loop {
            if pos == 0 {
                break;
            }
            pos -= 1;
            if self.is_navigable(pos) {
                self.selected = pos;
                break;
            }
        }
    }

    pub fn move_down(&mut self) {
        if self.display_rows.is_empty() {
            return;
        }
        let mut pos = self.selected;
        loop {
            if pos >= self.display_rows.len() - 1 {
                break;
            }
            pos += 1;
            if self.is_navigable(pos) {
                self.selected = pos;
                break;
            }
        }
    }

    pub fn move_top(&mut self) {
        for i in 0..self.display_rows.len() {
            if self.is_navigable(i) {
                self.selected = i;
                return;
            }
        }
    }

    pub fn move_bottom(&mut self) {
        for i in (0..self.display_rows.len()).rev() {
            if self.is_navigable(i) {
                self.selected = i;
                return;
            }
        }
    }

    pub fn selected_display_row(&self) -> Option<&DisplayRow> {
        self.display_rows.get(self.selected)
    }

    pub fn selected_room(&self) -> Option<&RoomSummary> {
        match self.display_rows.get(self.selected) {
            Some(DisplayRow::Room { room_index, .. }) => self.rooms.get(*room_index),
            _ => None,
        }
    }

    pub fn toggle_space(&mut self) {
        if let Some(DisplayRow::SpaceHeader { space_id, .. }) = self.display_rows.get(self.selected)
        {
            let space_id = space_id.clone();
            let collapsed = self.collapsed_spaces.entry(space_id).or_insert(false);
            *collapsed = !*collapsed;
            self.rebuild_display_rows();
            if self.selected >= self.display_rows.len() {
                self.selected = self.display_rows.len().saturating_sub(1);
            }
            self.skip_to_navigable();
        }
    }

    pub fn set_filter(&mut self, query: Option<String>) {
        self.search_filter = query;
        self.rebuild_display_rows();
        if self.display_rows.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.display_rows.len() {
            self.selected = self.display_rows.len().saturating_sub(1);
        }
        if !self.display_rows.is_empty() {
            self.skip_to_navigable();
        }
    }
}
