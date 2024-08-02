# revise

A Rust library for hashing data structures and then caching results and avoiding work

---

The main component of the library is `RevisionHash`, which just an integer summary/digest of the contents of an arbitrary data structure through the `Revisable` trait and its `get_revision()` method. Through the use of hashing, when an object's `RevisionHash` is changed, we know the contents have changed, and conversely, if the `RevisionHash` is unchanged, with very high probability its contents also are unchanged. Thus, in a larger program, by storing a hash of some data model from a previous known good state, we can easily compare its new hash and accurately determine whether or not to perform an expensive update from the new data.

A basic implementation of the `Revisable` trait is as follows. An implementation should hash all fields that are relevant to the identity of the object and the contents it represents.

```rust
struct Test(i32, u8);

impl Revisable for Test {
    fn get_revision(&self) -> RevisionHash {
        let mut hasher = RevisionHasher::new();
        hasher.write_i32(self.0);
        hasher.write_u8(self.1);
        hasher.into_revision()
    }
}
```

Recursion is encouraged wherever objects contain sub-objects that also implement `Revisable`, simply with a line like `hasher.write_revisable(&self.subobject);`.

Care should be taken when dealing with enums (you should probably hash the discriminator first). Care should also be taken when hashing variable-size data structures like `Vec` and `HashMap`, although some blanket implementations already exist which simplify this.

A helper struct `Revised<T>` is provided which looks and acts like an object of type `T` but caches the `RevisionHash` of that object as needed whenever it isn't being mutably accessed. This is useful when composing larger data structure to ensure that not all contents need to be read and hashed during partial modifications or if no modifications have been made at all. Conceivably, this could also be used for lightweight equality comparisons of very large data structures.

Another helper struct `RevisedProperty<T>` is provided which represents a lazily-computed result of calling some function which returns `T` and whose arguments are all `Revisable`. Its methods `refreshN(f, arg0, arg1, ... argN)` will call `f` on the provided arguments only if the arguments if different from the last call to a `refresh` method. This can be used to avoid expensive computations while still ensuring that derived results are up-to-date. The result of calling `f` is available through the `get_cached()` method.

I may add other helpers when I feel clever and have a use for them.
