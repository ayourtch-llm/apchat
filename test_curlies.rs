// Test file for file_curly_glance

fn main() {
    let x = 5;
    println!("Hello, world!");
}

fn helper() {
    let y = 10;
}

struct MyStruct {
    field1: i32,
    field2: String,
}

impl MyStruct {
    fn new() -> Self {
        MyStruct {
            field1: 0,
            field2: String::new(),
        }
    }
}
