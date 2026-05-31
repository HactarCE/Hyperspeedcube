use std::any::Any;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::Arc;

use itertools::Itertools;

/// Marker trait for types that can be stored in an [`ComponentList`]`<T>`.
///
/// This trait has no methods. It is effectively equivalent to `'static + Send +
/// Sync + Any` except that it is restricted to types that intentionally
/// implement this trait.
pub trait Component<T>: 'static + Send + Sync + Any {}

/// Map containing up to a single value of each type, restricted to types that
/// implement the marker trait [`Component`]`<T>`.
///
/// This is implemented internally using `Vec<Box<dyn Send + Sync + Any>>`, so
/// it is best used with relatively few items in the list.
///
/// Each contained type is wrapped in an [`Arc`].
pub struct ComponentList<T> {
    /// `&'static str` is a type name for debug purposes; it's not used for
    /// comparisons.
    entries: Vec<(&'static str, Arc<dyn Send + Sync + Any>)>,
    _marker: PhantomData<T>,
}

impl<T: 'static> Default for ComponentList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static> Debug for ComponentList<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypeMap")
            .field(
                "entries",
                &self
                    .entries
                    .iter()
                    .map(|(type_name, _)| type_name)
                    .collect_vec(),
            )
            .finish()
    }
}

impl<T> Clone for ComponentList<T> {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T: 'static> ComponentList<T> {
    /// Constructs an empty map.
    pub const fn new() -> Self {
        Self {
            entries: vec![],
            _marker: PhantomData,
        }
    }

    /// Returns the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Adds or overwrites the entry for type `E`.
    pub fn insert<E: Component<T>>(&mut self, value: Arc<E>) {
        let new_entry = (
            std::any::type_name::<E>(),
            value as Arc<dyn Send + Sync + Any>,
        );
        match self.entries.iter().position(|(_, e)| e.is::<E>()) {
            Some(i) => self.entries[i] = new_entry,
            None => self.entries.push(new_entry),
        }
    }

    /// Returns the entry for type `E`, or `None` if it is not in the map.
    pub fn get<E: Component<T>>(&self) -> Result<Arc<E>, MissingComponent> {
        self.entries
            .iter()
            .filter(|(_, e)| e.is::<E>())
            .find_map(|(_, e)| Arc::clone(e).downcast().ok()) // should always succeed
            .ok_or(MissingComponent::new::<T, E>())
    }

    /// Returns a reference to the entry for type `E`, or `None` if it is not in
    /// the map.
    pub fn get_ref<E: Component<T>>(&self) -> Result<&E, MissingComponent> {
        self.entries
            .iter()
            .filter(|(_, e)| e.is::<E>())
            .find_map(|(_, e)| e.downcast_ref()) // should always succeed
            .ok_or(MissingComponent::new::<T, E>())
    }

    /// Returns whether there is an entry for type `E`.
    pub fn contains<E: Component<T>>(&self) -> bool {
        self.entries
            .iter()
            .any(|entry| (entry as &dyn Any).is::<Arc<E>>())
    }
}

/// Error type returned for a missing entry in an [`ComponentList`].
#[derive(thiserror::Error, Debug)]
#[error("this {object_type} does not have component {component_type}")]
pub struct MissingComponent {
    object_type: &'static str,
    component_type: &'static str,
}

impl MissingComponent {
    fn new<T: 'static, E>() -> Self {
        let object_type = std::any::type_name::<T>();
        Self {
            object_type: match object_type
                .rsplit_once("::")
                .filter(|_| !object_type.contains('<'))
            {
                Some((_, last_component)) => last_component,
                None => object_type,
            },
            component_type: std::any::type_name::<E>(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestComponent;

    impl Component<crate::Puzzle> for TestComponent {}

    #[test]
    fn test_component_list() {
        let mut component_list = ComponentList::<crate::Puzzle>::new();
        assert_eq!(0, component_list.len());

        // Test inserting into blank slot
        component_list.insert(Arc::new(TestComponent));
        assert!(component_list.get::<TestComponent>().is_ok());
        assert_eq!(1, component_list.len());

        // Test inserting into existing slot
        component_list.insert(Arc::new(TestComponent));
        assert!(component_list.get::<TestComponent>().is_ok());
        assert_eq!(1, component_list.len());
    }
}
