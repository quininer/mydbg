#![feature(used_with_arg)]

use std::hint::black_box;


#[derive(Debug)]
#[allow(dead_code)]
struct MyObject {
    name: String
}

mydbg_debug::derive!(MyObject);
mydbg_debug::r#static!(MyObject, STATIC_MYDEBUG_MYOBJECT);

fn new_myobject() -> Box<MyObject> {
    Box::new(black_box(MyObject {
        name: "myobj".into()
    }))
}

#[inline(never)]
fn foo(obj: &MyObject) {
    black_box(obj);
}

fn main() {
    let obj = new_myobject();

    foo(&obj);
}
