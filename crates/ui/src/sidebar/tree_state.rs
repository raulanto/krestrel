//! Pure tree state management, folder expansion, node selection, and list flattening.

use crate::sidebar::search::fuzzy_match;

use kestrel_core::{CollectionItem, HttpMethod};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlatTreeNode {
    pub id: String,
    pub name: String,
    pub depth: usize,
    pub kind: FlatNodeKind,
    pub matched_indices: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlatNodeKind {
    Folder {
        is_collapsed: bool,
        has_children: bool,
    },
    Request {
        method: HttpMethod,
        url: String,
    },
    ErrorNode {
        file_path: PathBuf,
        error_message: String,
        line: Option<usize>,
        column: Option<usize>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct TreeState {
    pub collapsed_folders: HashSet<String>,
    pub selected_id: Option<String>,
    pub focused_index: usize,
    pub search_query: String,
}

impl TreeState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn toggle_folder(&mut self, folder_id: &str) {
        if self.collapsed_folders.contains(folder_id) {
            self.collapsed_folders.remove(folder_id);
        } else {
            self.collapsed_folders.insert(folder_id.to_string());
        }
    }

    pub fn expand_folder(&mut self, folder_id: &str) {
        self.collapsed_folders.remove(folder_id);
    }

    pub fn collapse_folder(&mut self, folder_id: &str) {
        self.collapsed_folders.insert(folder_id.to_string());
    }

    pub fn set_search_query(&mut self, query: &str) {
        self.search_query = query.to_string();
    }

    pub fn clear_search_query(&mut self) {
        self.search_query.clear();
    }

    /// Flattens the collection tree into a visible list of `FlatTreeNode` items.
    pub fn flatten(&self, items: &[CollectionItem]) -> Vec<FlatTreeNode> {
        let mut flat = Vec::new();
        let query = self.search_query.trim();

        if query.is_empty() {
            self.flatten_normal(items, 0, &mut flat);
        } else {
            self.flatten_filtered(items, query, 0, &mut flat);
        }

        flat
    }

    fn flatten_normal(&self, items: &[CollectionItem], depth: usize, out: &mut Vec<FlatTreeNode>) {
        for item in items {
            match item {
                CollectionItem::Folder(folder) => {
                    let is_collapsed = self.collapsed_folders.contains(&folder.id);
                    let has_children = !folder.items.is_empty();
                    out.push(FlatTreeNode {
                        id: folder.id.clone(),
                        name: folder.name.clone(),
                        depth,
                        kind: FlatNodeKind::Folder {
                            is_collapsed,
                            has_children,
                        },
                        matched_indices: Vec::new(),
                    });

                    if !is_collapsed {
                        self.flatten_normal(&folder.items, depth + 1, out);
                    }
                }
                CollectionItem::Request(req) => {
                    out.push(FlatTreeNode {
                        id: req.id.clone(),
                        name: req.name.clone(),
                        depth,
                        kind: FlatNodeKind::Request {
                            method: req.method,
                            url: req.url.clone(),
                        },
                        matched_indices: Vec::new(),
                    });
                }
                CollectionItem::ErrorNode(err) => {
                    out.push(FlatTreeNode {
                        id: err.id.clone(),
                        name: err.name.clone(),
                        depth,
                        kind: FlatNodeKind::ErrorNode {
                            file_path: err.file_path.clone(),
                            error_message: err.error_message.clone(),
                            line: err.line,
                            column: err.column,
                        },
                        matched_indices: Vec::new(),
                    });
                }
            }
        }
    }

    fn flatten_filtered(
        &self,
        items: &[CollectionItem],
        query: &str,
        depth: usize,
        out: &mut Vec<FlatTreeNode>,
    ) -> bool {
        let mut any_matched = false;

        for item in items {
            match item {
                CollectionItem::Folder(folder) => {
                    let folder_match = fuzzy_match(&folder.name, query);
                    let mut child_out = Vec::new();
                    let children_matched =
                        self.flatten_filtered(&folder.items, query, depth + 1, &mut child_out);

                    if folder_match.is_some() || children_matched {
                        any_matched = true;
                        let matched_indices =
                            folder_match.map(|m| m.matched_indices).unwrap_or_default();

                        out.push(FlatTreeNode {
                            id: folder.id.clone(),
                            name: folder.name.clone(),
                            depth,
                            kind: FlatNodeKind::Folder {
                                is_collapsed: false, // Force expanded during search
                                has_children: !folder.items.is_empty(),
                            },
                            matched_indices,
                        });

                        out.extend(child_out);
                    }
                }
                CollectionItem::Request(req) => {
                    let name_match = fuzzy_match(&req.name, query);
                    let url_match = fuzzy_match(&req.url, query);

                    if let Some(m) = name_match.or(url_match) {
                        any_matched = true;
                        out.push(FlatTreeNode {
                            id: req.id.clone(),
                            name: req.name.clone(),
                            depth,
                            kind: FlatNodeKind::Request {
                                method: req.method,
                                url: req.url.clone(),
                            },
                            matched_indices: m.matched_indices,
                        });
                    }
                }
                CollectionItem::ErrorNode(err) => {
                    if let Some(m) = fuzzy_match(&err.name, query) {
                        any_matched = true;
                        out.push(FlatTreeNode {
                            id: err.id.clone(),
                            name: err.name.clone(),
                            depth,
                            kind: FlatNodeKind::ErrorNode {
                                file_path: err.file_path.clone(),
                                error_message: err.error_message.clone(),
                                line: err.line,
                                column: err.column,
                            },
                            matched_indices: m.matched_indices,
                        });
                    }
                }
            }
        }

        any_matched
    }

    pub fn move_up(&mut self, total_count: usize) {
        if total_count == 0 {
            self.focused_index = 0;
        } else if self.focused_index > 0 {
            self.focused_index -= 1;
        }
    }

    pub fn move_down(&mut self, total_count: usize) {
        if total_count == 0 {
            self.focused_index = 0;
        } else if self.focused_index + 1 < total_count {
            self.focused_index += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_core::{Folder, Request};

    fn sample_tree() -> Vec<CollectionItem> {
        vec![
            CollectionItem::Folder(Folder {
                id: "f-users".into(),
                name: "Users".into(),
                description: None,
                seq: Some(1),
                items: vec![
                    CollectionItem::Request(Request {
                        id: "r-list".into(),
                        name: "List Users".into(),
                        method: HttpMethod::GET,
                        url: "https://api.com/users".into(),
                        seq: Some(1),
                        headers: vec![],
                        params: vec![],
                        auth: Default::default(),
                        body: Default::default(),
                        description: None,
                        path: None,
                        dirty: false,
                        extra: Default::default(),
                    }),
                    CollectionItem::Request(Request {
                        id: "r-create".into(),
                        name: "Create User".into(),
                        method: HttpMethod::POST,
                        url: "https://api.com/users".into(),
                        seq: Some(2),
                        headers: vec![],
                        params: vec![],
                        auth: Default::default(),
                        body: Default::default(),
                        description: None,
                        path: None,
                        dirty: false,
                        extra: Default::default(),
                    }),
                ],
                path: None,
                dirty: false,
                extra: Default::default(),
            }),
            CollectionItem::Request(Request {
                id: "r-health".into(),
                name: "Health Check".into(),
                method: HttpMethod::GET,
                url: "https://api.com/health".into(),
                seq: Some(2),
                headers: vec![],
                params: vec![],
                auth: Default::default(),
                body: Default::default(),
                description: None,
                path: None,
                dirty: false,
                extra: Default::default(),
            }),
        ]
    }

    #[test]
    fn test_flatten_expanded_vs_collapsed() {
        let tree = sample_tree();
        let mut state = TreeState::new();

        // Initially expanded
        let flat1 = state.flatten(&tree);
        assert_eq!(flat1.len(), 4); // Users folder, List Users, Create User, Health Check

        // Collapse Users folder
        state.collapse_folder("f-users");
        let flat2 = state.flatten(&tree);
        assert_eq!(flat2.len(), 2); // Users folder, Health Check
        assert_eq!(flat2[0].name, "Users");
        assert_eq!(flat2[1].name, "Health Check");
    }

    #[test]
    fn test_search_filtering_and_expansion_preservation() {
        let tree = sample_tree();
        let mut state = TreeState::new();
        state.collapse_folder("f-users"); // User collapsed folder

        // Search for "create"
        state.set_search_query("create");
        let flat_filtered = state.flatten(&tree);
        assert_eq!(flat_filtered.len(), 2); // Parent folder Users + Create User
        assert_eq!(flat_filtered[0].name, "Users");
        assert_eq!(flat_filtered[1].name, "Create User");

        // Clear search -> Users folder should still be collapsed per user state!
        state.clear_search_query();
        let flat_cleared = state.flatten(&tree);
        assert_eq!(flat_cleared.len(), 2); // Users folder + Health Check
    }
}
