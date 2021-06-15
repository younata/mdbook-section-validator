# mdbook-section-validator

![[CI status](https://ci.younata.com/teams/main/pipelines/knowledge/jobs/mdbook-section-validator/)](https://ci.younata.com/api/v1/pipelines/knowledge/jobs/mdbook-section-validator/badge)

mdbook preprocessor for providing sections that are conditionally valid/invalid.

Useful for providing information on workarounds that are subject to get fixed.

## Getting Started

First, install the `mdbook-section-validator` crate

```
cargo install mdbook-section-validator
```

Then, add the following line to your `book.toml` file

```toml
[preprocessor.section-validator]
```

Finally, add some custom CSS styling to control how the section validation sections work, I use the following in [personal-knowledge](https://github.com/younata/personal_knowledge/blob/master/css/custom.css):

```css
.validated-content {
    background-color: var(--quote-bg);
    border: 1px solid var(--quote-border);
    padding: 0 16px;
}
```

Once done, you can now use `!!!` at the beginning of lines to define sections to include only if the issues linked are still valid.

After the `!!!` starting a conditional inclusion, you must include a comma-separated list of URLs to tickets.
If all the linked issues are still open, then the markdown in the section will be included. The section is either removed, or prepended with a note stating that the section is out of date. 

```
# Chapter 1

!!!https://github.com/younata/mdbook-section-validator/issues/1,https://github.com/younata/mdbook-section-validator/issues/2

This is only rendered while issues 1 and 2 of younata/mdbook-section-validator are open. If 1 is closed, but not 2, then this is removed the next time mdbook renders. 
!!!

This is always rendered.
```

The inner contents will be rendered as markdown when running `mdbook build` or `mdbook serve` as usual.

## Configuration

As noted, this defaults to removing sections that are no longer valid. You can configure that be overriding `hide_invalid` to false.
You can configure the message to be shown by setting `invalid_message` to any string. It will be rendered as markdown.

```toml
[preprocessor.section-validator]
hide_invalid = false
invalid_message = "Warning, this content is out of date and is included for historical reasons."
```