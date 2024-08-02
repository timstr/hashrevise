use std::{
    cell::Cell,
    collections::HashMap,
    hash::Hasher,
    ops::{BitXor, Deref, DerefMut},
};

#[cfg(test)]
mod test;

/// RevisionHasher is an efficient hasher used to compute revision hashes.
pub struct RevisionHasher {
    hasher: seahash::SeaHasher,
}

impl RevisionHasher {
    /// Construct a new RevisionHasher
    pub fn new() -> RevisionHasher {
        RevisionHasher {
            hasher: seahash::SeaHasher::new(),
        }
    }

    /// Recursively hash another object and write its resulting
    /// RevisionHash
    pub fn write_revisable<T: Revisable>(&mut self, t: &T) {
        self.write_revision(t.get_revision());
    }

    /// Hash the RevisionHash of another object
    pub fn write_revision(&mut self, r: RevisionHash) {
        self.hasher.write_u64(r.value());
    }

    /// Consume the RevisionHasher and return its final RevisionHash
    /// which summarizes the contents it has seen
    pub fn into_revision(self) -> RevisionHash {
        RevisionHash::new(self.hasher.finish())
    }
}

impl Hasher for RevisionHasher {
    fn finish(&self) -> u64 {
        self.hasher.finish()
    }

    fn write(&mut self, bytes: &[u8]) {
        self.hasher.write(bytes);
    }
}

/// RevisionHash is an integer summary of the contents of a data structure,
/// based on hashing, intended to be used in distinguishing whether data
/// structures have changed or not.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct RevisionHash(u64);

impl RevisionHash {
    /// Create a new RevisionHash with the given integer value
    pub fn new(value: u64) -> RevisionHash {
        RevisionHash(value)
    }

    /// Get the integer value of the RevisionHash
    pub fn value(&self) -> u64 {
        self.0
    }
}

/// Revisable is a trait for types for which a RevisionHash can be computed.
/// Something that implements Revisable can have changes to its contents
/// tracked by watching its RevisionHash alone.
pub trait Revisable {
    /// Compute the RevisionHash of the object's contents. This should hash
    /// together everything that is relevant to the meaning of the object's
    /// contents and should be a pure function, i.e. it should produce the
    /// exact same result if the object is unchanged or has been changed to
    /// something which is semantically identical.
    fn get_revision(&self) -> RevisionHash;
}

/// Blanket implementation for references
impl<T> Revisable for &T
where
    T: Revisable + ?Sized,
{
    fn get_revision(&self) -> RevisionHash {
        T::get_revision(self)
    }
}

/// Blanket implementation for 1-tuples
impl<T> Revisable for (T,)
where
    T: Revisable,
{
    fn get_revision(&self) -> RevisionHash {
        self.0.get_revision()
    }
}

/// Blanket implementation for 2-tuples
impl<T0, T1> Revisable for (T0, T1)
where
    T0: Revisable,
    T1: Revisable,
{
    fn get_revision(&self) -> RevisionHash {
        let mut hasher = RevisionHasher::new();
        hasher.write_revision(self.0.get_revision());
        hasher.write_revision(self.1.get_revision());
        RevisionHash::new(hasher.finish())
    }
}

/// Blanket implementation for 3-tuples
impl<T0, T1, T2> Revisable for (T0, T1, T2)
where
    T0: Revisable,
    T1: Revisable,
    T2: Revisable,
{
    fn get_revision(&self) -> RevisionHash {
        let mut hasher = RevisionHasher::new();
        hasher.write_revision(self.0.get_revision());
        hasher.write_revision(self.1.get_revision());
        hasher.write_revision(self.2.get_revision());
        RevisionHash::new(hasher.finish())
    }
}

/// Revised is a wrapper struct for efficiently tracking the RevisionHash of
/// a desired type T. The RevisionHash is computed lazily and is only
/// recomputed when the object has been accessed mutably. This is achieved
/// transparently using the Deref and DerefMut traits such that Revised<T>
/// behaves in code just like a plain old T, except that computing its
/// RevisionHash is optimized to avoid redundant recursions through all its
/// contents to compute hash values, at the cost of a little extra storage.
#[derive(Clone)]
pub struct Revised<T> {
    /// The stored object
    value: T,

    /// The revision hash of the stored object, if it's up to date
    revision: Cell<Option<RevisionHash>>,
}

impl<T: Revisable> Revised<T> {
    /// Construct a new Revised object containing the given object
    pub fn new(value: T) -> Revised<T> {
        Revised {
            value,
            revision: Cell::new(None),
        }
    }

    /// Get the contained object's RevisionHash. If the object is
    /// not mutated, this will compute the RevisionHash only once
    /// and cache it for reuse.
    pub fn get_revision(&self) -> RevisionHash {
        match self.revision.get() {
            Some(v) => v,
            None => {
                let v = self.value.get_revision();
                self.revision.set(Some(v));
                v
            }
        }
    }
}

/// Revised<T> can deref to &T
impl<T: Revisable> Deref for Revised<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

/// Revised<T> can deref to &mut T
impl<T: Revisable> DerefMut for Revised<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.revision.set(None);
        &mut self.value
    }
}

/// Revised<T> is Revisable (obviously?)
impl<T: Revisable> Revisable for Revised<T> {
    fn get_revision(&self) -> RevisionHash {
        Revised::get_revision(&self)
    }
}

/// [T] where T is Revisable is also Revisable
impl<T> Revisable for [T]
where
    T: Revisable,
{
    fn get_revision(&self) -> RevisionHash {
        let mut hasher = RevisionHasher::new();

        // Hash the length first
        hasher.write_usize(self.len());

        // Hash the individual items
        for item in self {
            hasher.write_revision(item.get_revision());
        }

        RevisionHash::new(hasher.finish())
    }
}

/// HashMap<K, T> where K and T are both Revisable is also Revisable
impl<K, T> Revisable for HashMap<K, T>
where
    K: Revisable,
    T: Revisable,
{
    fn get_revision(&self) -> RevisionHash {
        // Get an order-independent hash of all items
        let mut items_hash: u64 = 0;
        for (key, value) in self {
            let mut item_hasher = RevisionHasher::new();
            item_hasher.write_u8(0x1);
            item_hasher.write_revision(key.get_revision());
            item_hasher.write_u8(0x2);
            item_hasher.write_revision(value.get_revision());
            // Use xor to combine hashes of different items so as
            // to not depend on the order of items in the hash map
            items_hash = items_hash.bitxor(item_hasher.finish());
        }

        let mut hasher = seahash::SeaHasher::new();

        // Hash the length first
        hasher.write_usize(self.len());

        // Add the hash value of all items
        hasher.write_u64(items_hash);

        RevisionHash::new(hasher.finish())
    }
}

/// RevisedVec<T> is shorthand for Vec<Revised<T>>.
pub type RevisedVec<T> = Vec<Revised<T>>;

/// RevisedHashMap<K, T> is shorthand for HashMap<K, Revised<T>>.
pub type RevisedHashMap<K, T> = HashMap<K, Revised<T>>;

/// Revised property is the cached result of a function call that is only
/// evaluated lazily whenever the inputs have changed, according to their
/// RevisionHash.
pub struct RevisedProperty<T> {
    /// The revision of the arguments for the cached value, if present
    revision: Option<RevisionHash>,

    /// The cached value
    value: Option<T>,
}

impl<T> RevisedProperty<T> {
    /// Create a new RevisedProperty with an empty cache
    pub fn new() -> RevisedProperty<T> {
        RevisedProperty {
            revision: None,
            value: None,
        }
    }

    /// Get the cached value, which might not be filled yet.
    /// This stores the result of the refresh* method that
    /// was most recently called, if any.
    pub fn get_cached(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// Update the cache to store the result of calling f(arg0).
    /// If the function's output from the same arguments is already
    /// cached, the function is not called and the cache is kept.
    /// Otherwise, f is called and the cache is written to.
    /// f is assumed to be a pure function.
    pub fn refresh1<F, A0>(&mut self, f: F, arg0: A0)
    where
        F: Fn(A0) -> T,
        A0: Revisable,
    {
        let current_revision = arg0.get_revision();
        if self.revision != Some(current_revision) {
            self.value = Some(f(arg0));
            self.revision = Some(current_revision);
        }
    }

    /// Update the cache to store the result of calling f(arg0, arg1).
    /// If the function's output from the same arguments is already
    /// cached, the function is not called and the cache is kept.
    /// Otherwise, f is called and the cache is written to.
    /// f is assumed to be a pure function.
    pub fn refresh2<F, A0, A1>(&mut self, f: F, arg0: A0, arg1: A1)
    where
        F: Fn(A0, A1) -> T,
        A0: Revisable,
        A1: Revisable,
    {
        let current_revision = (&arg0, &arg1).get_revision();
        if self.revision != Some(current_revision) {
            self.value = Some(f(arg0, arg1));
            self.revision = Some(current_revision);
        }
    }

    /// Update the cache to store the result of calling f(arg0, arg1, arg2).
    /// If the function's output from the same arguments is already
    /// cached, the function is not called and the cache is kept.
    /// Otherwise, f is called and the cache is written to.
    /// f is assumed to be a pure function.
    pub fn refresh3<F, A0, A1, A2>(&mut self, f: F, arg0: A0, arg1: A1, arg2: A2)
    where
        F: Fn(A0, A1, A2) -> T,
        A0: Revisable,
        A1: Revisable,
        A2: Revisable,
    {
        let current_revision = (&arg0, &arg1, &arg2).get_revision();
        if self.revision != Some(current_revision) {
            self.value = Some(f(arg0, arg1, arg2));
            self.revision = Some(current_revision);
        }
    }
}
