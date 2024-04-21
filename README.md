# FnMap
FnMap is a abstraction around the HashMap, like TypeMap. But uses closure's type(each closure's type is *unique* in Rust) as key and stores produced *value*. Allowing to be used like effective low cost dependency injection container.

## Usage
```rust
use fn_map::FnMap;

let map = FnMap::new();

fn one() -> i32 {
    println!("one computed");
    1
}

// get or compute(and insert) value using given closure. The closure depends on value of `one` function to compute its output.
let a = *map.get(|| map.get(one) + 1);
dbg!(a);

// b is *not* a because each closure's type is unique
let b = *map.get(|| map.get(one) + 1);
dbg!(b);

// get or compute(and insert) value using give function. But will not compute since it is computed already when producing a.
let c = *map.get(one);
dbg!(c);
```

will output
```bash
one computed
a = 2
b = 2
c = 1
```

# License
MIT
