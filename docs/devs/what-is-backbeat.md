---
title: What is Backbeat?
---

Backbeat is a high-performance, simple standard for storing and sharing rhythm game
charts, difficulty tables, packs and courses.

It is intended for _finished_, _released_ charts. It is not intended for - and will not work on -
charts that are actively being changed.

If you're someone who plays rhythm games and doesn't develop for them, you can [download the Backbeat application](https://github.com/zkldi/backbeat/releases) for
your system. We support Linux, MacOS and Windows. Having the backbeat app installed lets you click on
`Download with Backbeat` links.

From there, a good mental model is that you have a special `Backbeat Charts/` folder on your PC that any game can read from, if they integrate with backbeat.

## Why?

At the moment, downloading and sharing content for most at-home rhythm games is
miserable. Charts are uploaded to google drive, torrents, dead mediafire links,
discord channels, and so on.

It's extremely difficult to just download these games and get started - some
games bake in systems (such as osu!) which are nice, but they don't generalise
across games. If USC wanted to have a nice osu style system, they'd have to
implement it themselves.

Backbeat provides a system for downloading and managing charts. It
can be integrated into any open-source game, with just one library and a couple hundred lines of code.
