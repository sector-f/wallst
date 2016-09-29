# wallst
A **WALL**paper **S**e**T**ter written in Rust

## SYNOPSIS

**wallst** [**--hflip**] [**--vflip**] [**--color** *COLOR*] [**--output** *FILENAME*] [**--mode** *MODE*] *IMAGE*

## USAGE

**IMAGE** is necessary unless `--color` is used. If **IMAGE** is `-`,
**wallst** will read from standard input rather than from a file.

**COLOR** must be in the form `#RRGGBB`. If no color is specified, it defaults to `#000000`. If more than one color is specified, a gradient is created with the specified
colors going from left to right.

The following **MODE**s are available:

* center - The image is centered on the screen.

* stretch - The image is stretched to fit the screen.

* fill - The image is scaled until it fits in the screen, and is then centered.

* full - The image is placed in the top-left of the screen.

* tile - The image is tiled if it is smaller than the screen.

With all modes, excluding **stretch**, aspect ratio is preserved,
and the image is surrounded by the background color if necessary.

`--output` is used to save the image (with any modifications) as a PNG.

`--hflip` and `--vflip`, unsurprisingly, flip the image
horizontally and vertically, respectively.

## Thanks

* [meh](https://github.com/meh/) for both creating a
[Rust binding to the xcb-util library](https://github.com/meh/rust-xcb-util) and for helping me out on IRC

* [kori](https://github.com/kori/) for coming up with the name "wallst"
(which I suppose can be pronounced either "wall street" or "wall set")
