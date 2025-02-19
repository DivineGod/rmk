//! Initialize default keymap from config
//!

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

use crate::keyboard_config::KeyboardConfig;

/// Read the default keymap setting in `keyboard.toml` and add as a `get_default_keymap` function
pub(crate) fn expand_layout_init(keyboard_config: &KeyboardConfig) -> TokenStream2 {
    let mut layers = vec![];
    for layer in keyboard_config.layout.keymap.clone() {
        layers.push(expand_layer(layer));
    }
    return quote! {
        pub const fn get_default_keymap() -> [[[::rmk::action::KeyAction; COL]; ROW]; NUM_LAYER] {
            [#(#layers), *]
        }
    };
}

/// Push rows in the layer
fn expand_layer(layer: Vec<Vec<String>>) -> TokenStream2 {
    let mut rows = vec![];
    for row in layer {
        rows.push(expand_row(row));
    }
    quote! { [#(#rows), *] }
}

/// Push keys in the row
fn expand_row(row: Vec<String>) -> TokenStream2 {
    let mut keys = vec![];
    for key in row {
        keys.push(parse_key(key));
    }
    quote! { [#(#keys), *] }
}

struct ModifierCombinationMacro {
    right: bool,
    gui: bool,
    alt: bool,
    shift: bool,
    ctrl: bool,
}
impl ModifierCombinationMacro {
    fn new() -> Self {
        Self {
            right: false,
            gui: false,
            alt: false,
            shift: false,
            ctrl: false,
        }
    }
    fn is_empty(&self) -> bool {
        !(self.gui || self.alt || self.shift || self.ctrl)
    }
}
// Allows to use `#modifiers` in the quote
impl quote::ToTokens for ModifierCombinationMacro {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let right = self.right;
        let gui = self.gui;
        let alt = self.alt;
        let shift = self.shift;
        let ctrl = self.ctrl;

        tokens.extend(quote! {
            ::rmk::keycode::ModifierCombination::new_from(#right, #gui, #alt, #shift, #ctrl)
        });
    }
}

/// Get modifier combination, in types of mod1 | mod2 | ...
fn parse_modifiers(modifiers_str: &str) -> ModifierCombinationMacro {
    let mut combination = ModifierCombinationMacro::new();
    let tokens = modifiers_str.split_terminator("|");
    tokens.for_each(|w| {
        let w = w.trim();
        match w {
            "LShift" => combination.shift = true,
            "LCtrl" => combination.ctrl = true,
            "LAlt" => combination.alt = true,
            "LGui" => combination.gui = true,
            "RShift" => {
                combination.right = true;
                combination.shift = true;
            }
            "RCtrl" => {
                combination.right = true;
                combination.ctrl = true;
            }
            "RAlt" => {
                combination.right = true;
                combination.alt = true;
            }
            "RGui" => {
                combination.right = true;
                combination.gui = true;
            }
            _ => (),
        }
    });
    combination
}

/// Parse the key string at a single position
fn parse_key(key: String) -> TokenStream2 {
    if key.len() < 5 {
        return if key.len() > 0 && key.trim_start_matches("_").len() == 0 {
            quote! { ::rmk::a!(No) }
        } else {
            let ident = format_ident!("{}", key);
            quote! { ::rmk::k!(#ident) }
        };
    }
    match &key[0..3] {
        "WM(" => {
            if let Some(internal) = key.trim_start_matches("WM(").strip_suffix(")") {
                let keys: Vec<&str> = internal
                    .split_terminator(",")
                    .map(|w| w.trim())
                    .filter(|w| w.len() > 0)
                    .collect();
                if keys.len() != 2 {
                    return quote! {
                        compile_error!("keyboard.toml: WM(layer, modifier) invalid, please check the documentation: https://haobogu.github.io/rmk/keyboard_configuration.html");
                    };
                }

                let ident = format_ident!("{}", keys[0].to_string());

                let modifiers = parse_modifiers(keys[1]);

                if modifiers.is_empty() {
                    return quote! {
                        compile_error!("keyboard.toml: modifier in WM(layer, modifier) is not valid! Please check the documentation: https://haobogu.github.io/rmk/keyboard_configuration.html");
                    };
                }
                quote! {
                    ::rmk::wm!(#ident, #modifiers)
                }
            } else {
                return quote! {
                    compile_error!("keyboard.toml: WM(layer, modifier) invalid, please check the documentation: https://haobogu.github.io/rmk/keyboard_configuration.html");
                };
            }
        }
        "MO(" => {
            let layer = get_layer(key, "MO(", ")");
            quote! {
                ::rmk::mo!(#layer)
            }
        }
        "OSL" => {
            let layer = get_layer(key, "OSL(", ")");
            quote! {
                ::rmk::osl!(#layer)
            }
        }
        "OSM" => {
            if let Some(internal) = key.trim_start_matches("OSM(").strip_suffix(")") {
                let modifiers = parse_modifiers(internal);

                if modifiers.is_empty() {
                    return quote! {
                        compile_error!("keyboard.toml: modifier in OSM(modifier) is not valid! Please check the documentation: https://haobogu.github.io/rmk/keyboard_configuration.html");
                    };
                }
                quote! {
                    ::rmk::osm!(#modifiers)
                }
            } else {
                quote! {
                    compile_error!("keyboard.toml: OSM(modifier) invalid, please check the documentation: https://haobogu.github.io/rmk/keyboard_configuration.html");
                }
            }
        }
        "LM(" => {
            if let Some(internal) = key.trim_start_matches("LM(").strip_suffix(")") {
                let keys: Vec<&str> = internal
                    .split_terminator(",")
                    .map(|w| w.trim())
                    .filter(|w| w.len() > 0)
                    .collect();
                if keys.len() != 2 {
                    return quote! {
                        compile_error!("keyboard.toml: LM(layer, modifier) invalid, please check the documentation: https://haobogu.github.io/rmk/keyboard_configuration.html");
                    };
                }
                let layer = keys[0].parse::<u8>().unwrap();

                let modifiers = parse_modifiers(keys[1]);

                if modifiers.is_empty() {
                    return quote! {
                        compile_error!("keyboard.toml: modifier in LM(layer, modifier) is not valid! Please check the documentation: https://haobogu.github.io/rmk/keyboard_configuration.html");
                    };
                }
                quote! {
                    ::rmk::lm!(#layer, #modifiers)
                }
            } else {
                return quote! {
                    compile_error!("keyboard.toml: LM(layer, modifier) invalid, please check the documentation: https://haobogu.github.io/rmk/keyboard_configuration.html");
                };
            }
        }
        "LT(" => {
            let keys: Vec<&str> = key
                .trim_start_matches("LT(")
                .trim_end_matches(")")
                .split_terminator(",")
                .map(|w| w.trim())
                .filter(|w| w.len() > 0)
                .collect();
            if keys.len() != 2 {
                return quote! {
                    compile_error!("keyboard.toml: LT(layer, key) invalid, please check the documentation: https://haobogu.github.io/rmk/keyboard_configuration.html");
                };
            }
            let layer = keys[0].parse::<u8>().unwrap();
            let key = format_ident!("{}", keys[1]);
            quote! {
                ::rmk::lt!(#layer, #key)
            }
        }
        "TT(" => {
            let layer = get_layer(key, "TT(", ")");
            quote! {
                ::rmk::tt!(#layer)
            }
        }
        "TG(" => {
            let layer = get_layer(key, "TG(", ")");
            quote! {
                ::rmk::tg!(#layer)
            }
        }
        "TO(" => {
            let layer = get_layer(key, "TO(", ")");
            quote! {
                ::rmk::to!(#layer)
            }
        }
        "DF(" => {
            let layer = get_layer(key, "DF(", ")");
            quote! {
                ::rmk::df!(#layer)
            }
        }
        "MT(" => {
            if let Some(internal) = key.trim_start_matches("MT(").strip_suffix(")") {
                let keys: Vec<&str> = internal
                    .split_terminator(",")
                    .map(|w| w.trim())
                    .filter(|w| w.len() > 0)
                    .collect();
                if keys.len() != 2 {
                    return quote! {
                        compile_error!("keyboard.toml: MT(key, modifier) invalid, please check the documentation: https://haobogu.github.io/rmk/keyboard_configuration.html");
                    };
                }
                let ident = format_ident!("{}", keys[0].to_string());
                let modifiers = parse_modifiers(keys[1]);

                if modifiers.is_empty() {
                    return quote! {
                        compile_error!("keyboard.toml: modifier in MT(key, modifier) is not valid! Please check the documentation: https://haobogu.github.io/rmk/keyboard_configuration.html");
                    };
                }
                quote! {
                    ::rmk::mt!(#ident, #modifiers)
                }
            } else {
                return quote! {
                    compile_error!("keyboard.toml: MT(key, modifier) invalid, please check the documentation: https://haobogu.github.io/rmk/keyboard_configuration.html");
                };
            }
        }
        "TH(" => {
            if let Some(internal) = key.trim_start_matches("TH(").strip_suffix(")") {
                let keys: Vec<&str> = internal
                    .split_terminator(",")
                    .map(|w| w.trim())
                    .filter(|w| w.len() > 0)
                    .collect();
                if keys.len() != 2 {
                    return quote! {
                        compile_error!("keyboard.toml: TH(key_tap, key_hold) invalid, please check the documentation: https://haobogu.github.io/rmk/keyboard_configuration.html");
                    };
                }
                let ident1 = format_ident!("{}", keys[0].to_string());
                let ident2 = format_ident!("{}", keys[1].to_string());

                quote! {
                    ::rmk::th!(#ident1, #ident2)
                }
            } else {
                return quote! {
                    compile_error!("keyboard.toml: TH(key_tap, key_hold) invalid, please check the documentation: https://haobogu.github.io/rmk/keyboard_configuration.html");
                };
            }
        }
        _ => {
            let ident = format_ident!("{}", key);
            quote! {::rmk::k!(#ident) }
        }
    }
}

/// Parse the string literal like `MO(1)`, `OSL(1)`, get the layer number in it.
/// The caller should pass the trimmed prefix and suffix
fn get_layer(key: String, prefix: &str, suffix: &str) -> u8 {
    let layer_str = key.trim_start_matches(prefix).trim_end_matches(suffix);
    layer_str.parse::<u8>().unwrap()
}
