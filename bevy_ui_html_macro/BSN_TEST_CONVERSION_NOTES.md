# BSN Test Conversion Notes

Status of converting the `legacy` test module to the `bsn` test module in
`src/lib.rs`, verified with
`cargo nextest -p bevy_ui_html_macro --features=bsn,propagate`.

## Result

All remaining legacy tests were converted and pass. No tests had to be
commented out.

Converted in this pass (all previously missing from the `bsn` module):

- `html_component::self_closing_custom_tag_no_tuple`
- `html_component::extra_string_attrs_forwarded_to_additional_attributes`
- `html_component::multiple_extra_attrs_all_forwarded`
- `html_component::standard_component_attrs_passed_inside_html_bundle`
- `html_component::rust_expression_extra_attrs_not_in_additional_attributes`
- `feathers_elements::renders_button`
- `feathers_elements::spawns_observer`
- `feathers_elements::button_supports_overrides`
- `feathers_elements::renders_checkbox`
- `feathers_elements::renders_radio`

All other legacy tests already had BSN counterparts that pass.

## Caveats and observations (not fixed, per instructions)

### `feathers_elements` tests do not run under the specified command

They are gated behind `#[cfg(feature = "feathers")]`, which is not in
`--features=bsn,propagate`, so the given command compiles them out (64 tests).
They were verified with an additional run using
`--features=bsn,propagate,feathers` (69 tests, all passing).

### Feathers output is not BSN-converted in the source

`button_bundle` / `checkbox_bundle` / `radio_bundle` calls are emitted
identically in legacy and BSN modes (no `#[cfg(feature = "bsn")]` branch in
`src/components/feathers/`). Inside `bsn!{}` this produces mixed output that
the token-comparison tests accept but looks suspect at runtime:

- Button children use the BSN child form, so a text child becomes
  `((bevy::ui::widget::Text("Hello")))` as a plain function argument to
  `button_bundle` — outside a `bsn!` `Children[...]` context that is just a
  parenthesized tuple-struct call, not a spawnable `Spawn(...)` entry.
- Checkbox/radio labels and all observers still emit legacy runtime forms
  (`::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new(...))`,
  `::bevy::ecs::spawn::SpawnWith(...)`) embedded inside the `bsn!{}` block.

The converted tests assert the current actual output; if the feathers path is
later ported to BSN properly, these expectations will need updating.

### Observers inside `bsn!{}` use legacy `SpawnWith`

Same pattern in the pre-existing `bsn::children::adds_observers` test: observer
attachment is emitted as `::bevy::ecs::spawn::SpawnWith(...)` inside
`Children[...]`. Whether `bsn!` accepts that at runtime is untested here (these
are token-string tests only).

### Sibling crate `bevy_ui_html` integration tests are NOT converted

19 integration tests (`bevy_ui_html/tests/html.rs`: 7,
`tests/html_component.rs`: 12) have no BSN gating and fail to compile under
`cargo nextest -p bevy_ui_html --features=bsn,propagate`. In BSN mode `html!`
expands to `bsn!{...}`, which evaluates to a
`SceneScope<SceneFunction<...>>` rather than a spawnable bundle, so
`world.spawn(html!{...})` and component assertions type-mismatch. Converting
these is not a mechanical rewrite — it needs a decision on how BSN scenes are
spawned/asserted in tests (and possibly runtime support in `bevy_ui_html`).

### HtmlBundle `node` field formatting in BSN mode

Inside `<_ as ::bevy_ui_html::HtmlComponent>::build(...)`, the `node:` field is
emitted in BSN struct form (`bevy::ui::Node` / `bevy::ui::Node { ... }` with no
`..Default::default()`), even though `HtmlBundle` is constructed as a plain
Rust struct expression. As plain Rust this would not compile without
`..Default::default()`; it presumably relies on `bsn!` re-interpreting the
tokens. Tests assert the current output.
