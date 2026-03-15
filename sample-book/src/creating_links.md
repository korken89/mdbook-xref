# Creating links

This preprocessor allow you to create cross-referenceable links by creating new link items
that use the `label` protocol. Additionally, it lets you set the supplement (the text substituted
at the place of reference) for these links, which is used unless the referenced text is specified
explicitly.

## Creating cross-referenceable labels

A piece of referenceable text can be created as follows:

```
[a piece of referenceable text](label:a_piece_of_text "an optional supplement")
```

which is rendered like this:

[a piece of referenceable text](label:a_piece_of_text "an optional supplement")


## Cross-referencing non-text items

Non-text items can also be cross-referenced as follows:

```
<table id="non-text">
    <th>A header</th>
</table>

[](label:non-text "Table 1")
```

The trick here is that cross-references to `non-text` will create a link to the created table.

In general, you should not create cross-referenceable items like this. Instead, you should let
other preprocessors generated them, and run the crossref preprocessor after them.

It is rendered like this (note that the labelled data is not explicitly visible):

<table id="non-text">
    <th>A header</th>
</table>

[](label:non-text "Table 1")

## Referring to cross-referefences { #creating }
[](label:creating "the section on creating references")

A cross reference can be referred to by any links prefixed with the `ref` protocol:

```
# Autolinks will only work if the reference specified a supplement.
<ref:a_piece_of_text>

# Normal inline links also work
[A reference to the text.](label:a_piece_of_text)
```

These references are rendered as follows:

<ref:a_piece_of_text>

[A reference to the text.](ref:a_piece_of_text)

We can also refer to the table created earlier:

<ref:non-text>