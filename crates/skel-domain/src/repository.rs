use crate::{DomainError, Item, ItemId};

/// Port interface for entity storage.
///
/// Decouples pure domain state machines from physical persistence engines
/// (in-memory maps, NVMe log-structured merge trees, or remote data stores).
pub trait ItemRepository {
    /// Store a new item. Fails with [`DomainError::Duplicate`] if the id exists.
    fn insert(&mut self, item: Item) -> Result<(), DomainError>;

    /// Replace an existing item. Fails with [`DomainError::NotFound`] if absent.
    fn update(&mut self, item: Item) -> Result<(), DomainError>;

    /// Fetch an item by id.
    fn get(&self, id: &ItemId) -> Result<Item, DomainError>;

    /// List all items, ordered by id.
    fn list(&self) -> Vec<Item>;
}
