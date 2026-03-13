## A heading `buh` <label:buh>

## This is a second, properly labelelled heading <label:a_second_heading>

### <label:in_between> but labels not at the end are ignored

```typst
#set page(height: auto, width: auto, margin: 1em)
#import "@preview/chronos:0.2.1"
#chronos.diagram({
  import chronos: *
  _par("Alice")
  _par("Bob")

  _seq("Alice", "Bob", comment: "Hello")
  _seq("Bob", "Bob", comment: "Think")
  _seq("Bob", "Alice", comment: "Hi")
})
```

Some text in between

<typst:nested/test.typ>

Some more text in between

<typst:/nested/test.typ>

As mentioned in <ref:buh>