# Getting started with GuiQL

##

```sh
$ guiql
query >
```

```sh
query > CREATE Dialog "Hello, world!"
```

```sh
query > CREATE @window Window {}
@window
query > CREATE @main VStack {}
@main
query > CREATE Label "Hello, world!" INTO @main
@main/a1b95dd3-fe35-4626-aaa8-351452dd5f34
query > DEFINE Button extends Div (
      >   label = ""
      > ) {
      >   Div label { style = { padding = 10px, background = "#1f1f1f", } }
      > }
query > CREATE @btn Button "0" INTO @main
@btn
query > SUBSCRIBE @btn.click
query > REPLACE @btn.label = "1"
```