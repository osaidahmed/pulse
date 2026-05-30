fn build_list(n: usize) -> Vec<u32> {
    let mut items = vec![0u32; n];
    for i in 0..n {
        items[i] = i as u32;
    }
    items
}

fn report(items: &[u32]) {
    for item in items {
        println!("item: {}", item);
    }
    assert!(!items.is_empty());
}

fn formatted(name: &str) -> String {
    format!("hello, {name}")
}
