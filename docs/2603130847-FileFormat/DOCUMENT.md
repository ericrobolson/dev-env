# On File Formats

**Source:** [solhsa.com](https://solhsa.com/oldernews2025.html#ON-FILE-FORMATS)

Ten practical guidelines for designing file formats:

1. **Use existing formats first.** Check if your community already has a standard before inventing one.

2. **Decide on human readability.** Text formats (JSON, XML, INI) are easier to parse. If binary, keep specs unambiguous — use `0`/`1` over error-prone strings like `"Disabled"`.

3. **Use chunked binary architecture.** Chunk-based structures (tag + length per chunk) enable nesting, concatenation, and extensibility. Used in .3DS, .FLI, .AIF, RIFF.

4. **Support partial parsing.** Let tools extract specific sections without processing the entire file.

5. **Include a version field.** Cheap to add, prevents future pain — even for formats you think won't change.

6. **Document thoroughly.** Specs must allow reimplementation by others (or yourself, years later). Ambiguity invites bugs.

7. **Avoid speculative fields.** Don't pad for hypothetical futures. Use pointers to subchunks for clean expansion in later versions.

8. **Consider target hardware.** Match field sizes and endianness to the target architecture. Embedded devices need different approaches than desktops.

9. **Weigh compression tradeoffs.** Consider: whole-file vs. per-chunk compression, modification patterns, memory limits, seeking needs, and decompression speed.

10. **Check filename extensions.** Verify your chosen extension isn't already taken. Four-letter extensions have more availability than three-letter ones.
