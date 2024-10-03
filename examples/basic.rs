use sigil_rs::Sigil;
use sigil_rs::Theme;

fn main() {
    let theme = Theme::default();

    let sigil = Sigil::generate(&theme, "example");

    sigil.to_image(240).save("example.png").unwrap();
}
