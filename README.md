# FnStore
A dynamic persistent value store using closure type as key and storing its return value

## Usage
```rust
use fn_store::LocalFnStore;

let mut store = LocalFnStore::new();

let a = *store.get(|| 1);
let b = *store.get(|| 2);

assert_eq!(a, 1);
assert_eq!(b, 2);
```

# License
MIT
