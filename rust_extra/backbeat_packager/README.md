# `backbeat_packager`

The backbeat packager takes external game content and packages them into `.bb` or `.bbzip` files.

## What's in here?

We have support for stepmania, bms and kshoot at the moment. Tentatively `.tja` and `.dtx`, but not yet.

## Quick Primer

A `.bb` file is a chart file, wrapped with a mapping of `filename -> sha256`. To create these,
you need to do logic like this:

```ts
let chart = parseChartFile(filename);
let assets = {}

for filename referenced by chart:
	assets[filename] = sha256(readFile(filename))

return {
	filename,
	assets,
	chart: base64(gzip(readFile(chartFilename)))
}
```

This is just pseudocode though, and you have to also:

1. know how to parse the charts for the game to find what it references
2. know how the game does referencing. For example, bms allows `foo.wav` to be satisfied by `foo.opus`, stepmania does clever stuff, and case-insensitivity is always fun.

## How to write simple packagers.

If you want to write your own packager, you're lazy, and **you know for certain that your game does not support `../music.mp3` pathing**, you can do:

```ts
let assets = {};

for filename in dir:
	if filename == chartFilename:
		skip

	assets[filename] = sha256(readFile(filename))

return {
	filename: chartFilename,
	assets,
	chart: base64(gzip(readFile(chartFilename)))
}
```

## What's a `.bbzip` file?

A `.bbzip` file is a renamed zip file. They're a convenient way of shipping around charts for last mile traffic.
They look like this:

```
whatever.wav
somethingelse.ogg
dontcare.txt
foo.bb
```

The filenames are irrelevant. Tools can then use `.bbzip` like this:

```ts
for file, contents in bbzip {
	if file.endsWith(".bb") {
		addBundle(contents)
	} else {
		addAsset(contents)
	}
}
```

It's just a very useful way of shipping around a chart and its dependencies.
