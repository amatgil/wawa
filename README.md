# wawa
A relatively basic discord bot for prettifying code

It is run with prefix commands: `wawa!<cmd> <args>` or `w!<cmd> <args>`

# Usage
Use either of the prefixes or a direct mention followed by the command. For example:
- `wawa!ping`: pong
- `wawa!run <code>`: run the code (can be in ascii!)
- `wawa!docs <fn>`: print the documentation for a function
- `wawa!pad <code>`: format the code and provide a pad link


# Full list of commands
- ping: pong
- h / help: display this text!
- v / ver / version: display uiua version used by the rest of commands
- f / fmt: run the formatter
- p / pad: format and generate a link to the pad
- d / docs <fn>: show the first paragraph or so of the specified function
- r / run: format and run the code
- e / emojify: converts the given code to discord emoji as best as possible


# Examples
TODO: This was populated but fell out of date

All uiua codeblocks use custom syntax highlighting using the `ansi` environment (which is
quite limited, discord does not offer many much of the ansi spec).

# Goals
- [X] Docs command: `w! docs tuple` returns the documentation for `tuple`
- [X] `w! pad <code>` Automatic pad link
- [X] have `fmt` color glyphs
- [X] have `fmt`'s colors look good
- [X] Catch messages that are too long
- [X] Audio embeds (don't just crash)
- [ ] Image embeds (don't just crash)
- [ ] Gif embeds (don't just crash)
- [X] Short summary of function in `w! docs`
- [X] Write out help
- [X] Accept `w!cmd` syntax
- [X] Automate command dispatch
- [X] Add `tracing`
- [X] Detect raw pad links and sent it wrapped (Check if the string contains `https://uiua.org/pad?src` and not `(https://uiua.org/pad?src`)
- [ ] Handle single backticks by checking if the last char is a backtick (can't negate an empty stack). same with three
- [ ] Slash commands (example 5)
- [ ] `w!docs changelog`
- [ ] Fix internal links in documentation (like in `under`'s docs), probably by regex subst
- [ ] Add space and time constraints for `w!run`
