# sigil

sigil is a minimal and fast identicon generator.

```
cargo run --example basic
```
![](./example.png)

sigil is compatible with, and ports some code from, [Cupcake Sigil].

[Cupcake Sigil]: https://github.com/tent/sigil

## Stability

sigil-rs works today and can be used as is without ever upgrading. I use sigil-rs in several web apps.

Before callling it 1.0, I want to:
- Produce smaller PNGs. Sigils are currently about 1-2KB, but pngquant brings them down to < 200 bytes, so it's not perfect.
- Maybe produce PNGs in a single pass with no intermediate allocations?

## Alternatives

- [identicon-rs] is the most popular identicon generator. It is very flexible, but far less efficient.
- [plot_icon] generates a different image style that's quite cute.

[identicon-rs]: https://github.com/conways-glider/identicon-rs
[plot_icon]: https://github.com/paritytech/polkadot-identicon-rust

## License

[BSD-3-Clause](./LICENSE)
