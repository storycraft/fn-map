use fn_map::FnMap;

fn main() {
    let map = FnMap::new();

    fn one() -> i32 {
        println!("one computed");
        1
    }
    
    let a = *map.get(|| map.get(one) + 1);
    dbg!(a);
    assert_eq!(a, 2);

    let b = *map.get(|| map.get(one) + 1);
    dbg!(b);
    assert_eq!(b, 2);

    let c = *map.get(one);
    dbg!(c);
    assert_eq!(c, 1);
}
