# late_init
Partially initialize fields with the rest filled as defaults. 

Provides a `LateInit` derive macro which brings in scope a `struct NameLateInit` created from `struct Name` definition.

Then it can be used as follows:
```rust
#[derive(Defailt)]
struct HasDefault;
struct NoDefault;

#[derive(LateInit)]
struct Name {
    a: HasDefault,
    b: NoDefault,
    c: HasDefault,
    /*insert multiple field defs similar to a*/
    x: HasDefault,
    y: HasDefault,
    z: NoDefault,
}

fn create_name() -> Name {
    NameLateInit::default()
        .b(NoDefault) // You have to provide fields which do not have default impl
        .x(HasDefault) // These are optional
        .z(NoDefault)
        .finish()
}
```
May as well serve as an alternative to derive-new.

#

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
