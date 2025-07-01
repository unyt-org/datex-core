# Diagrams

All diagrams are generated using [D2](https://d2lang.com/).

The layout engine used is `ELK`.
The theme used for all diagrams is a custom theme based on `Dark mauve`.
To apply the theme in a `.d2` file, add the following snippet at the top:

```d2
vars: {
    d2-config: {
        theme-overrides: {
            B1: "#D44778"
            B2: "#D44778"
            B3: "#6C7086"
            B4: "#585B70"
            B5: "#45475A"
            B6: "#313244"

            AA2: "#77BED7"
            AA4: "#45475A"
            AA5: "#313244"

            AB4: "#45475A"
            AB5: "#313244"
        }
    }
}
```

**Online Playground:** https://play.d2lang.com/

## Installing D2

```sh
curl -fsSL https://d2lang.com/install.sh | sh -s --
```

## Creating diagrams with live reload

```sh
 d2 d2/diagram.d2 -t 200 --watch
```

## Rendering diagrams

```sh
deno run -A render.ts
```