# wawa
A relatively basic discord bot for prettifying code

It is run with prefix commands: `!wawa <cmd> <args>` or `!w <cmd> <args>`

# Usage
- `!wawa ping`: pong
- `!wawa run <code>`: run the code (can be in ascii!)
- `!wawa docs <fn>`: print the documentation for a function
- `!wawa pad <code>`: format the code and provide a pad link


# Examples
`!w run unshape 2_3_4` returns 
```
╭─             
╷  0  1  2  3  
╷  4  5  6  7  
   8  9 10 11  
               
  12 13 14 15  
  16 17 18 19  
  20 21 22 23  
              ╯
```

`!w docs shape` returns 
```
Get the dimensions of an array

△5 # []
△[] # [0]
△1_2_3 # [3]
△[1_2 3_4 5_6] # [3 2]
```
`!w pad unshape roundmul10rand_rand_rand` returns 
```
[Pad](https://uiua.org/pad?src=0_13_0-rc_1__wrDilrMg4oGFw5cxMOKagl_imoIK) for:
```uiua
°△ ⁅×10⚂_⚂
\```
```

All uiua codeblocks use custom syntax highlighting using the `ansi` environment (which is
incredibly limited, thank you discord for using the smallest number of colors possible).

# Goals
- [X] Docs command: `w! docs tuple` returns the documentation for `tuple`
- [X] `w! pad <code>` Automatic pad link
- [X] have `fmt` color glyphs
- [X] have `fmt`'s colors look good
- [X] Catch messages that are too long
- [ ] Audio embeds (don't just crash)
- [ ] Image embeds (don't just crash)
- [ ] Gif embeds (don't just crash)
- [ ] Short summary of function in `w! docs`
- [ ] Write out help
- [ ] Accept `w!cmd` syntax
- [ ] Automate command dispatch
- [ ] Add `tracing` (including reason, span, date, etc)
- [ ] Accept single backtick code
- [ ] Detect raw pad links and sent it wrapped (Check if the string contains `https://uiua.org/pad?src` and not `(https://uiua.org/pad?src`)
