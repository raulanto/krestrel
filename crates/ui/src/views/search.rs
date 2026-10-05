use kestrel_core::{Collection, CollectionItem, Folder, HttpMethod, Request};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilteredItem {
    pub id: String,
    pub name: String,
    pub kind: FilteredItemKind,
    pub path: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilteredItemKind {
    Folder,
    Request(HttpMethod),
}

/// Recursively filters collection items by search query (case-insensitive).
/// If query is empty, returns all items preserving folder hierarchy paths.
pub fn filter_collection_items(
    items: &[CollectionItem],
    query: &str,
    current_path: &[String],
) -> Vec<FilteredItem> {
    let clean_query = query.trim().to_lowercase();
    let mut results = Vec::new();

    for item in items {
        match item {
            CollectionItem::Folder(Folder {
                id,
                name,
                items: sub_items,
                ..
            }) => {
                let mut path_with_folder = current_path.to_vec();
                path_with_folder.push(name.clone());

                let folder_matches =
                    clean_query.is_empty() || name.to_lowercase().contains(&clean_query);
                let sub_results = filter_collection_items(sub_items, query, &path_with_folder);

                if folder_matches || !sub_results.is_empty() {
                    results.push(FilteredItem {
                        id: id.clone(),
                        name: name.clone(),
                        kind: FilteredItemKind::Folder,
                        path: current_path.to_vec(),
                    });
                    results.extend(sub_results);
                }
            }
            CollectionItem::Request(Request {
                id,
                name,
                method,
                url,
                ..
            }) => {
                let name_matches = clean_query.is_empty()
                    || name.to_lowercase().contains(&clean_query)
                    || url.to_lowercase().contains(&clean_query);

                if name_matches {
                    results.push(FilteredItem {
                        id: id.clone(),
                        name: name.clone(),
                        kind: FilteredItemKind::Request(*method),
                        path: current_path.to_vec(),
                    });
                }
            }
        }
    }

    results
}

/// Helper that filters all items from a collection root.
pub fn filter_collection(collection: &Collection, query: &str) -> Vec<FilteredItem> {
    filter_collection_items(&collection.items, query, &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_collection_empty_query() {
        let collection = Collection {
            id: "col-1".into(),
            name: "My API".into(),
            description: None,
            items: vec![
                CollectionItem::Folder(Folder {
                    id: "f-1".into(),
                    name: "Auth".into(),
                    description: None,
                    items: vec![CollectionItem::Request(Request {
                        id: "r-1".into(),
                        name: "Login".into(),
                        method: HttpMethod::POST,
                        url: "https://api.test/login".into(),
                        headers: vec![],
                        params: vec![],
                        auth: Default::default(),
                        body: Default::default(),
                        description: None,
                        extra: Default::default(),
                    })],
                    extra: Default::default(),
                }),
                CollectionItem::Request(Request {
                    id: "r-2".into(),
                    name: "Health".into(),
                    method: HttpMethod::GET,
                    url: "https://api.test/health".into(),
                    headers: vec![],
                    params: vec![],
                    auth: Default::default(),
                    body: Default::default(),
                    description: None,
                    extra: Default::default(),
                }),
            ],
            environments: vec![],
            extra: Default::default(),
        };

        let items = filter_collection(&collection, "");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].name, "Auth");
        assert_eq!(items[1].name, "Login");
        assert_eq!(items[2].name, "Health");
    }

    #[test]
    fn test_filter_collection_with_query() {
        let collection = Collection {
            id: "col-1".into(),
            name: "My API".into(),
            description: None,
            items: vec![
                CollectionItem::Folder(Folder {
                    id: "f-1".into(),
                    name: "Auth".into(),
                    description: None,
                    items: vec![CollectionItem::Request(Request {
                        id: "r-1".into(),
                        name: "Login".into(),
                        method: HttpMethod::POST,
                        url: "https://api.test/login".into(),
                        headers: vec![],
                        params: vec![],
                        auth: Default::default(),
                        body: Default::default(),
                        description: None,
                        extra: Default::default(),
                    })],
                    extra: Default::default(),
                }),
                CollectionItem::Request(Request {
                    id: "r-2".into(),
                    name: "Get Users".into(),
                    method: HttpMethod::GET,
                    url: "https://api.test/users".into(),
                    headers: vec![],
                    params: vec![],
                    auth: Default::default(),
                    body: Default::default(),
                    description: None,
                    extra: Default::default(),
                }),
            ],
            environments: vec![],
            extra: Default::default(),
        };

        let results = filter_collection(&collection, "user");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Get Users");

        let results_folder = filter_collection(&collection, "log");
        assert_eq!(results_folder.len(), 2); // Includes folder Auth because child matched
        assert_eq!(results_folder[0].name, "Auth");
        assert_eq!(results_folder[1].name, "Login");
    }
}
