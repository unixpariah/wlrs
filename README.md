# WLRS /wɔːlrəs/ [![crate](https://img.shields.io/crates/v/wlrs.svg)](https://crates.io/crates/wlrs) [![Build Status](https://github.com/unixpariah/wlrs/actions/workflows/tests.yml/badge.svg)](https://github.com/unixpariah/wlrs/actions/workflows/tests.yml) [![codecov](https://codecov.io/gh/unixpariah/wlrs/graph/badge.svg?token=49LRWZ9D1K)](https://codecov.io/gh/unixpariah/wlrs) [![docs](https://docs.rs/wlrs/badge.svg)](https://docs.rs/wlrs/latest/wlrs/index.html) 

Rust library for managing wallpapers

## Supported environments

- Every Wayland compositor that implements layer-shell (e.g. Hyprland, Sway, Wayfire, etc.)
- X11 environments that don't have their own wallpaper management (e.g. dwm, i3, bspwm, etc.)

## Examples:

Setting from memory:
```rust
use wlrs::set_from_memory;

fn main() {
  // Set to first monitor
  let wallpaper = image::open("wallpaper.jpg").unwrap();
  set_from_memory(wallpaper, vec![0]).unwrap();

  // Set to multiple monitors
  let wallpaper = image::open("wallpaper.jpg").unwrap();
  set_from_memory(wallpaper, vec![0, 1]).unwrap();

  // Set to all monitors
  let wallpaper = image::open("wallpaper.jpg").unwrap();
  set_from_memory(wallpaper, Vec::new()).unwrap();
}
```

Setting from file path:

```rust
use wlrs::set_from_path;

fn main() {
  // Set to first monitor
  set_from_path("wallpaper.jpg", vec![0]).unwrap();

  // Set to multiple monitor
  set_from_path("wallpaper.jpg", vec![0, 1]).unwrap();
  
  // Set to all monitors
  set_from_path("wallpaper.jpg", Vec::new())unwrap();
}
```
