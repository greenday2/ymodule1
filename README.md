## Module one project work.

Parse bank transaction files in multiple formats.
Supported formats: bin, txt, csv.
See specs/ for format descriptions and examples.

## Utilities:

### converter - Parse and convert bank transcatons data from one of supported formats to another.

Example of using

```
cat file1.csv > converter --dest-format txt
```

Use ` converter --help ` for more information.


### comparer - Parse and compare two bank transaction files.

Example of using

```
comparer file1.txt file2.bin
```

Use ` comparer --help ` for more information.
