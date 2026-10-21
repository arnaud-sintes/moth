# Styling

Every moth component is a Web Component: its element is in your page, and its inside (the actual `<button>`, `<input>`...) is in a **shadow root**, a private subtree. That gives three levels of styling, from most to least convenient. What is not possible is styling the inside with a selector of your own: your page's CSS stops at the element.

## 1. The element: ordinary CSS

The component's element is a normal element of the page, so page CSS applies to it: size, margin, grid or flex placement, borders, background. Give it a class from Rust (`panel.element().set_class_name("card")`) or select by its tag (`moth-push-button`):

```css
.card { padding: 1rem; border: 1px solid #ccd; }
moth-label { font-weight: 600; }
```

Inherited properties (`color`, `font-family`, `font-size`...) also pass into the inside by themselves, since a shadow root inherits from its host.

## 2. CSS variables: the supported way to restyle the inside

Each component reads some CSS custom properties, with a default, and CSS variables **inherit into shadow roots**. Set them on `:root` to theme everything, on a class or an ancestor to theme part of the page, or on a single component's element to change only that one:

| Variable | Used by | Default |
|---|---|---|
| `--moth-button-color` | `PushButton`, `ToggleButton` | `inherit` |
| `--moth-button-background` | `PushButton`, `ToggleButton` | `ButtonFace` |
| `--moth-button-border` | `PushButton`, `ToggleButton` | the text colour |
| `--moth-button-padding` | `PushButton`, `ToggleButton` | `0.35em 0.9em` |
| `--moth-button-radius` | `PushButton`, `ToggleButton` | `4px` |
| `--moth-toggle-on-color` | `ToggleButton` while toggled | `Canvas` |
| `--moth-toggle-on-background` | `ToggleButton` while toggled | `CanvasText` |
| `--moth-input-color` | `Edit`, `Dropdown` | `inherit` |
| `--moth-input-background` | `Edit`, `Dropdown` | `Field` |
| `--moth-input-border` | `Edit`, `Dropdown` | the text colour |
| `--moth-input-padding` | `Edit`, `Dropdown` | `0.3em 0.5em` |
| `--moth-input-radius` | `Edit`, `Dropdown` | `4px` |
| `--moth-accent` | `Slider`, `ProgressBar` | the browser's accent colour |
| `--moth-panel-display` | `Panel` | `block` |
| `--moth-panel-gap` | `Panel` | `0` |
| `--moth-panel-align` | `Panel` | `normal` |
| `--moth-canvas-height` | `Canvas` | `150px` |

A `Panel` is a layout container: set `--moth-panel-display: flex` (or `grid`) and `--moth-panel-gap` and it lays its children out, with `flex-direction` and the rest set as ordinary CSS on its element.

Here is a theme that uses all three levels. The first rule themes the whole page, the second changes one button by changing its variables, the third uses a part:

```css
:root {
	--moth-accent: #e0457b;
	--moth-button-background: #fff0f5;
	--moth-button-border: #e0457b;
	--moth-button-radius: 999px;
	--moth-input-border: #e0457b;
	--moth-panel-display: flex;
	--moth-panel-gap: 0.5rem;
}

/* a button in a dark section, with nothing but its variables changed */
.danger {
	--moth-button-background: #7a1230;
	--moth-button-color: white;
}

/* what the variables do not reach: the inside of a component, through its parts */
moth-edit::part(input) {
	font-style: italic;
}
```

## 3. `::part()`: what the variables do not reach

Each component marks its main inner element with a `part`, which a page can select with `::part()` from the outside, for example `moth-edit::part(input) { font-style: italic; }`:

| Component | Part |
|---|---|
| `PushButton`, `ToggleButton` | `button` |
| `Label` | `text` |
| `Edit` | `input` |
| `Dropdown` | `select` |
| `Slider` | `slider` |
| `ProgressBar` | `progress` |
| `Image` | `image` |
| `Canvas` | `canvas` |

`::part()` can set any property of that element, but it cannot reach anything inside it, and it cannot use descendant selectors (`moth-edit::part(input):hover` is fine, `moth-edit::part(input) span` is not).

## What does not work

- **Global rules that name an inner element** (`button { ... }`, `input { ... }`, a CSS framework's `.btn`) do not reach into components, because the inner `<button>` is in a shadow root. Style the component's element, its variables, or its part.
- **Making a component use a framework's classes.** There is no opt-out of the shadow root yet (a light-DOM mode is an open item), so frameworks that style by class names cannot restyle moth's inner elements.
- **Disabled state** is drawn by the component (half opacity, `not-allowed` cursor); a `disabled` attribute on a component, or on a `<fieldset>` around it, applies to the whole subtree.

## Your own components

A component of your own, made from a `Definition` ([writing components](components.md#2-a-new-element-with-its-own-template-and-styles)), is styled the same way and should offer the same hooks: read your own variables with a default in your stylesheet (`background: var(--meter-fill, steelblue)`), and put a `part="..."` on the elements a page may want to restyle. Prefix your variables with your own name, not `--moth-`.

## Where the stock styles live

The stylesheets are `moth/src/elements/*.css` and the templates `*.html` next to them: short files, and the best reference for exactly what each component does.
