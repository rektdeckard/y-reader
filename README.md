# y-reader

Y Reader is a simple Hacker News client built in Rust using the [egui](https://github.com/emilk/egui) immediate-mode GUI toolkit. It's very much a work-in-progress, but features periodic refresh, comment browsing, and excessive use of concurrency to deal with the terrible Hacker News API design :)

<img src="/meta/y-reader-demo.gif" width="320" align="center" />

## Usage

Clone the repo and run:

```rs
cargo run --release
```

I plan to package some binaries once the project is a bit more stable!

## Planned Features

- [ ] Infinite scroll, don't limit to 100 items per tab
- [ ] Improved parsing of comment HTML
- [ ] Persistent custom UI settings
- [ ] Support `Ask` and `Jobs`
- [ ] In-app views for users
- [ ] Login, voting and commenting (no auth support yet from YC)

## License

MIT © [Tobias Fried](https://tobiasfried.com)
