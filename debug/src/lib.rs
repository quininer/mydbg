use std::fmt::Debug;

pub trait MyDebug: Debug {
    fn debug_to_stdout(&self);
}

#[macro_export]
macro_rules! derive {
    ( $ty:ty ) => {
        impl $crate::MyDebug for $ty {
            #[inline(never)]
            fn debug_to_stdout(&self) {
                $crate::debug_to_stdout(self);
            }
        }
    }
}

#[macro_export]
macro_rules! r#static {
    ( $( $ty:ty, $name:ident );* ) => {
        $(
            #[used(linker)]
            static $name: fn(&$ty) =
                <$ty as $crate::MyDebug>::debug_to_stdout;
        )*
    }
}

#[inline(never)]
pub fn debug_to_stdout(obj: &dyn Debug) {
    use std::io::{ self, Write };

    let mut stdout = io::stdout();
    let _ = writeln!(&mut stdout, "{:?}", obj);
}
