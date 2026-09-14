---
title: Backbeat
---

Backbeat is a high-performance, simple standard for storing and sharing rhythm game
charts, difficulty tables, packs and courses.

You can think of it like a special `Songs/` folder that is managed by a library instead of file writes/reads.

If you're a nerd: it's like Nix, if Nix worked, and was for rhythm game charts.

## Developer Documentation

These are docs for developers, mainly, but if you're curious about how this stuff works, you might find it useful too.

- [What is Backbeat?](./devs/what-is-backbeat.md)
- [How does Backbeat work?](./devs/how-it-works.md)
- [How do I integrate Backbeat into my game?](./devs/integrate-backbeat.md)
- [How do I run a Backbeat server?](./devs/running-a-server.md)

## Specifications

These are the raw, really detailed specs for Backbeat. They're not very friendly to read, but they are <i>the</i> reference if you're looking to implement your own things for backbeat (instead of just using the nice SDKs I've written for you :P)

- [Backbeat files](./specs/backbeat-file.md)
- [Backbeat zip files](./specs/backbeat-zip.md)
- [Tables](./specs/backbeat-table.md)
- [Courses](./specs/backbeat-course.md)
- [Packs](./specs/backbeat-pack.md)
- [Bundle IDs](./specs/bundle-id.md)
- [The Backbeat store](./specs/backbeat-store.md)
- [Backbeat configuration](./specs/backbeat-config.md)
- [Data servers](./specs/backbeat-server.md)
- [Collection endpoints](./specs/backbeat-collection-endpoint.md)
- [Combined assets](./specs/combined-assets.md)
- [Combined asset IDs](./specs/combined-assets-id.md)
