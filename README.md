# build

```
$ cargo bulid
$ c++ -std=c++14 wrapping/plugin.cpp -shared -fPIC ./target/debug/libmydbg.a -o libmydbg.so
```

and

```
$ cat $HOME/.lldbinit
plugin load /libmydbg.so
```
