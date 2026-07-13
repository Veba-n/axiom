# Refactor Plan: Modular Layer System
1. Remove `UiElement` monolithic visual properties (`fg_color`, `border`, `text`, `text_json`, etc).
2. Create `AxiomLayer` struct containing all visual properties + `StateOverride` per layer.
3. Update `UiElement` to hold `layers: Vec<AxiomLayer>`.
4. Update UI Panel to render a list of layers for the selected element, allowing adding/removing layers.
5. Update rendering logic to iterate over `el.layers`.
