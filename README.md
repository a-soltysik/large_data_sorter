# LargeDataSorter

Usage: 
```
large_data_sorter <command> [<args>]
```

Options:
```
--help            display usage information
```

Commands:
```
generator         generates a file with random u32 numbers
sorter            sorts a file using merge-sort algorithm
checker           checks if the given file is sorted
```

## Generator

`Usage: large_data_sorter generator -o <output-path> -n <numbers-count>`

generates a file with random u32 numbers

Options:
```
-o, --output-path   output path for generator
-n, --numbers-count u32 numbers count to be generated
--help              display usage information
```

## Sorter

Usage: 
```
large_data_sorter sorter -i <input-path> -o <output-path> [-t <threads-count>] [-s <data-in-ram>] [-e <exec-policy>]
```

sorts a file using merge-sort algorithm

Options:
```
-i, --input-path    path for input data to sort
-o, --output-path   path for sorted output
-t, --threads-count maximum threads count to be used during sorting
-s, --data-in-ram   maximum size of the file that can be sorted in ram
-e, --exec-policy   available values:
FullPar - sorting both files and in ram is parallel
FilePar - only sorting a file is parallel
RamPar  - only sorting in ram is parallel
--help              display usage information
```

## Checker

Usage: 
```
large_data_sorter checker -i <input-path>
```

checks if the given file is sorted

Options:
```
-i, --input-path  path of file to be checked
--help            display usage information
```
