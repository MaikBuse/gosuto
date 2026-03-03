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
    CallParticipant {
        display_name: String,
    },
}

fn extract_server_domain(room_id: &str) -> &str {
    room_id
        .split_once(':')
        .map(|(_, domain)| domain)
        .unwrap_or(room_id)
}

#[derive(Debug)]
pub struct RoomListState {
    pub rooms: Vec<RoomSummary>,
    pub display_rows: Vec<DisplayRow>,
    pub selected: usize,
    pub search_filter: Option<String>,
    pub room_call_members: HashMap<String, HashSet<String>>,
    collapsed_spaces: HashMap<String, bool>,
}

impl RoomListState {
    pub fn new() -> Self {
        Self {
            rooms: Vec::new(),
            display_rows: Vec::new(),
            selected: 0,
            search_filter: None,
            room_call_members: HashMap::new(),
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
                    if let Some(members) = self.room_call_members.get(&self.rooms[ci].id) {
                        let mut sorted_members: Vec<&String> = members.iter().collect();
                        sorted_members.sort();
                        for user_id in sorted_members {
                            let display_name = user_id
                                .strip_prefix('@')
                                .and_then(|s| s.split_once(':'))
                                .map(|(local, _)| local)
                                .unwrap_or(user_id)
                                .to_string();
                            rows.push(DisplayRow::CallParticipant { display_name });
                        }
                    }
                }
            }
        }

        // Orphan rooms: regular rooms not claimed by any space, grouped by server domain
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
            // Group orphans by server domain, preserving encounter order
            let mut domain_order: Vec<String> = Vec::new();
            let mut domain_groups: HashMap<String, Vec<usize>> = HashMap::new();
            for &oi in &orphans {
                let domain = extract_server_domain(&self.rooms[oi].id).to_string();
                domain_groups.entry(domain.clone()).or_default().push(oi);
                if !domain_order.contains(&domain) {
                    domain_order.push(domain);
                }
            }

            for domain in &domain_order {
                rows.push(DisplayRow::SectionHeader {
                    label: domain.clone(),
                });
                for &oi in &domain_groups[domain] {
                    rows.push(DisplayRow::Room {
                        room_index: oi,
                        indent: 0,
                    });
                    // Emit call participant rows for this room
                    if let Some(members) = self.room_call_members.get(&self.rooms[oi].id) {
                        let mut sorted_members: Vec<&String> = members.iter().collect();
                        sorted_members.sort();
                        for user_id in sorted_members {
                            let display_name = user_id
                                .strip_prefix('@')
                                .and_then(|s| s.split_once(':'))
                                .map(|(local, _)| local)
                                .unwrap_or(user_id)
                                .to_string();
                            rows.push(DisplayRow::CallParticipant { display_name });
                        }
                    }
                }
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
                if let Some(members) = self.room_call_members.get(&self.rooms[di].id) {
                    let mut sorted_members: Vec<&String> = members.iter().collect();
                    sorted_members.sort();
                    for user_id in sorted_members {
                        let display_name = user_id
                            .strip_prefix('@')
                            .and_then(|s| s.split_once(':'))
                            .map(|(local, _)| local)
                            .unwrap_or(user_id)
                            .to_string();
                        rows.push(DisplayRow::CallParticipant { display_name });
                    }
                }
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

    pub fn clear_unread(&mut self, room_id: &str) {
        if let Some(room) = self.rooms.iter_mut().find(|r| r.id == room_id) {
            room.unread_count = 0;
        }
        self.rebuild_display_rows();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn room(id: &str, name: &str, cat: RoomCategory) -> RoomSummary {
        RoomSummary {
            id: id.to_string(),
            name: name.to_string(),
            category: cat,
            unread_count: 0,
            is_space_child: false,
            parent_space_id: None,
        }
    }

    fn space_child(id: &str, name: &str, parent: &str) -> RoomSummary {
        RoomSummary {
            id: id.to_string(),
            name: name.to_string(),
            category: RoomCategory::Room,
            unread_count: 0,
            is_space_child: true,
            parent_space_id: Some(parent.to_string()),
        }
    }

    #[test]
    fn extract_server_domain_standard() {
        assert_eq!(extract_server_domain("!room:matrix.org"), "matrix.org");
    }

    #[test]
    fn extract_server_domain_no_colon() {
        assert_eq!(extract_server_domain("nocolon"), "nocolon");
    }

    #[test]
    fn extract_server_domain_with_port() {
        assert_eq!(
            extract_server_domain("!room:example.com:8448"),
            "example.com:8448"
        );
    }

    #[test]
    fn empty_rooms_list() {
        let state = RoomListState::new();
        assert!(state.display_rows.is_empty());
        assert!(state.selected_room().is_none());
    }

    #[test]
    fn rebuild_with_spaces_and_children() {
        let mut state = RoomListState::new();
        state.set_rooms(vec![
            room("!space1:x", "My Space", RoomCategory::Space),
            space_child("!child1:x", "Child Room", "!space1:x"),
        ]);
        // Should have: SpaceHeader, Room
        assert_eq!(state.display_rows.len(), 2);
        assert!(matches!(
            state.display_rows[0],
            DisplayRow::SpaceHeader { .. }
        ));
        assert!(matches!(
            state.display_rows[1],
            DisplayRow::Room { indent: 2, .. }
        ));
    }

    #[test]
    fn rebuild_orphan_rooms_grouped_by_domain() {
        let mut state = RoomListState::new();
        state.set_rooms(vec![
            room("!a:alpha.org", "Room A", RoomCategory::Room),
            room("!b:beta.org", "Room B", RoomCategory::Room),
            room("!c:alpha.org", "Room C", RoomCategory::Room),
        ]);
        // Should have: SectionHeader(alpha.org), Room A, Room C, SectionHeader(beta.org), Room B
        assert_eq!(state.display_rows.len(), 5);
        assert!(
            matches!(&state.display_rows[0], DisplayRow::SectionHeader { label } if label == "alpha.org")
        );
        assert!(matches!(
            state.display_rows[1],
            DisplayRow::Room {
                room_index: 0,
                indent: 0
            }
        ));
        assert!(matches!(
            state.display_rows[2],
            DisplayRow::Room {
                room_index: 2,
                indent: 0
            }
        ));
        assert!(
            matches!(&state.display_rows[3], DisplayRow::SectionHeader { label } if label == "beta.org")
        );
        assert!(matches!(
            state.display_rows[4],
            DisplayRow::Room {
                room_index: 1,
                indent: 0
            }
        ));
    }

    #[test]
    fn rebuild_dms_section() {
        let mut state = RoomListState::new();
        state.set_rooms(vec![
            room("!dm1:x", "Alice", RoomCategory::DirectMessage),
            room("!dm2:x", "Bob", RoomCategory::DirectMessage),
        ]);
        assert_eq!(state.display_rows.len(), 3);
        assert!(
            matches!(&state.display_rows[0], DisplayRow::SectionHeader { label } if label == "DIRECT MESSAGES")
        );
    }

    #[test]
    fn rebuild_search_filter() {
        let mut state = RoomListState::new();
        state.set_rooms(vec![
            room("!a:x", "General", RoomCategory::Room),
            room("!b:x", "Random", RoomCategory::Room),
        ]);
        state.set_filter(Some("gen".to_string()));
        // Only General should remain (plus its section header)
        let room_rows: Vec<_> = state
            .display_rows
            .iter()
            .filter(|r| matches!(r, DisplayRow::Room { .. }))
            .collect();
        assert_eq!(room_rows.len(), 1);
    }

    #[test]
    fn rebuild_collapsed_space_hides_children() {
        let mut state = RoomListState::new();
        state.set_rooms(vec![
            room("!space1:x", "Space", RoomCategory::Space),
            space_child("!child1:x", "Child", "!space1:x"),
        ]);
        assert_eq!(state.display_rows.len(), 2);
        // Select the space header and toggle
        state.selected = 0;
        state.toggle_space();
        // After collapse: only the SpaceHeader remains
        assert_eq!(state.display_rows.len(), 1);
        assert!(matches!(
            state.display_rows[0],
            DisplayRow::SpaceHeader {
                collapsed: true,
                ..
            }
        ));
        // Toggle again to expand
        state.selected = 0;
        state.toggle_space();
        assert_eq!(state.display_rows.len(), 2);
    }

    #[test]
    fn move_up_skips_section_headers() {
        let mut state = RoomListState::new();
        state.set_rooms(vec![
            room("!a:alpha.org", "Room A", RoomCategory::Room),
            room("!b:beta.org", "Room B", RoomCategory::Room),
        ]);
        // Layout: SectionHeader(alpha), Room A, SectionHeader(beta), Room B
        // Start at Room B (index 3)
        state.selected = 3;
        state.move_up();
        // Should skip SectionHeader(beta) and land on Room A (index 1)
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn move_down_skips_non_navigable() {
        let mut state = RoomListState::new();
        state.set_rooms(vec![
            room("!a:alpha.org", "Room A", RoomCategory::Room),
            room("!b:beta.org", "Room B", RoomCategory::Room),
        ]);
        // Start at Room A (index 1)
        state.selected = 1;
        state.move_down();
        // Should skip SectionHeader(beta) and land on Room B (index 3)
        assert_eq!(state.selected, 3);
    }

    #[test]
    fn move_down_skips_call_participants() {
        let mut state = RoomListState::new();
        let mut call_members = HashMap::new();
        call_members.insert("!a:x".to_string(), HashSet::from(["@user:x".to_string()]));
        state.room_call_members = call_members;
        state.set_rooms(vec![
            room("!a:x", "Room A", RoomCategory::Room),
            room("!b:x", "Room B", RoomCategory::Room),
        ]);
        // Layout: SectionHeader(x), Room A, CallParticipant, Room B
        // Find Room A's index
        let room_a_idx = state
            .display_rows
            .iter()
            .position(|r| matches!(r, DisplayRow::Room { room_index: 0, .. }))
            .unwrap();
        state.selected = room_a_idx;
        state.move_down();
        // Should skip the CallParticipant and land on Room B
        assert!(matches!(
            state.display_rows[state.selected],
            DisplayRow::Room { room_index: 1, .. }
        ));
    }

    #[test]
    fn move_top_finds_first_navigable() {
        let mut state = RoomListState::new();
        state.set_rooms(vec![
            room("!a:x", "Room A", RoomCategory::Room),
            room("!b:x", "Room B", RoomCategory::Room),
        ]);
        state.selected = 2;
        state.move_top();
        // First navigable is the SectionHeader(x) at 0? No — SectionHeader is not navigable.
        // Actually looking at is_navigable: SpaceHeader and Room are navigable.
        // SectionHeader is NOT navigable. So first navigable is Room A.
        assert!(matches!(
            state.display_rows[state.selected],
            DisplayRow::Room { room_index: 0, .. }
        ));
    }

    #[test]
    fn move_bottom_finds_last_navigable() {
        let mut state = RoomListState::new();
        state.set_rooms(vec![
            room("!a:x", "Room A", RoomCategory::Room),
            room("!b:x", "Room B", RoomCategory::Room),
        ]);
        state.move_bottom();
        assert!(matches!(
            state.display_rows[state.selected],
            DisplayRow::Room { room_index: 1, .. }
        ));
    }

    #[test]
    fn selected_room_on_room_row() {
        let mut state = RoomListState::new();
        state.set_rooms(vec![room("!a:x", "Room A", RoomCategory::Room)]);
        // Skip to first navigable (Room)
        state.move_top();
        let selected = state.selected_room().unwrap();
        assert_eq!(selected.name, "Room A");
    }

    #[test]
    fn selected_room_on_header_returns_none() {
        let mut state = RoomListState::new();
        state.set_rooms(vec![room("!a:x", "Room A", RoomCategory::Room)]);
        // Force selection to section header
        state.selected = 0;
        // SectionHeader is at index 0
        if matches!(state.display_rows[0], DisplayRow::SectionHeader { .. }) {
            assert!(state.selected_room().is_none());
        }
    }

    #[test]
    fn set_filter_clears_and_rebuilds() {
        let mut state = RoomListState::new();
        state.set_rooms(vec![
            room("!a:x", "Alpha", RoomCategory::Room),
            room("!b:x", "Beta", RoomCategory::Room),
        ]);
        let initial_len = state.display_rows.len();
        state.set_filter(Some("alpha".to_string()));
        assert!(state.display_rows.len() < initial_len);
        state.set_filter(None);
        assert_eq!(state.display_rows.len(), initial_len);
    }

    #[test]
    fn toggle_space_noop_on_non_space() {
        let mut state = RoomListState::new();
        state.set_rooms(vec![room("!a:x", "Room A", RoomCategory::Room)]);
        let initial_len = state.display_rows.len();
        state.move_top();
        state.toggle_space();
        assert_eq!(state.display_rows.len(), initial_len);
    }

    #[test]
    fn move_up_empty_display() {
        let mut state = RoomListState::new();
        state.move_up();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn move_down_empty_display() {
        let mut state = RoomListState::new();
        state.move_down();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn collapsed_space_aggregates_unread() {
        let mut state = RoomListState::new();
        let mut child = space_child("!child1:x", "Child", "!space1:x");
        child.unread_count = 5;
        state.set_rooms(vec![room("!space1:x", "Space", RoomCategory::Space), child]);
        state.selected = 0;
        state.toggle_space();
        if let DisplayRow::SpaceHeader { unread_count, .. } = &state.display_rows[0] {
            assert_eq!(*unread_count, 5);
        } else {
            panic!("Expected SpaceHeader");
        }
    }
}
