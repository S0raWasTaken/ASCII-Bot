<div align="center">

# ASCII Bot

  <img src="./koakuma_txt.png" alt="Original art by garasuno_1182" width="200"/>
  
**Turns any image into ASCII in a couple of clicks.**
</div>

### Commands:
- `/image_to_ascii <attachment> [charset]`
  - example: `/image_to_ascii .+p0#@` (assuming your message contains an attachment)
- `/attachment_to_ascii`
  - This one is a **context_menu_command**, which means you can right click on a message containing an attachment (links not supported *yet*) and run it through the Apps context menu.
- `/avatar_to_ascii`
  - Also a **context_menu_command**. Turns a user avatar into an ASCII image.

### Usage:
> If you're looking to use the bot right away, [click here](https://discord.com/oauth2/authorize?client_id=1441344772311613541) to add my current running version to your Discord User Apps.

#### Self hosting:
First of all, you're gonna need to compile your bot with a XOR'ed token file. For this, I provide the [xor_token.sh](/xor_token.sh) script, which is pretty simple to use:
```sh
# Use your actual bot token instead, of course.
# If you are unsure about how this works, read the script, it's all bash internals :)
./xor_token.sh "MTQ.EXAMPLE.TOKEN"
```
Make sure the file `.token.xor` exists in the project's root directory now.

From now on, it's simple rust compiling:
```sh
# You can either run it directly:
cargo run --release

# Or compile it and run the binary anywhere you want:
cargo build --release
# output: ./target/release/ascii-bot or ascii-bot.exe
```

### License
[MIT](LICENSE) and [OPEN FONT LICENSE](fonts/OFL.txt)

**TL;DR (not legal advice):**
- My project is free to use, modify and redistribute
- The font file is free to use, modify and redistribute, but don't sell it.
