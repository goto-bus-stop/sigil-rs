# sigil

sigil is a minimal and fast identicon generator.

```
cargo run --example basic
```
![](./example.png)

sigil is compatible with, and ports some code from, [Cupcake Sigil].

Before 1.0, I want to:
- Produce smaller PNGs
- Maybe produce PNGs in a single pass with no intermediate allocations?
- Maybe optionally produce SVGs?

[Cupcake Sigil]: https://github.com/tent/sigil

## License

[BSD-3-Clause](./LICENSE)
