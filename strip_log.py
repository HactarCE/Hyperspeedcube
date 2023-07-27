import re
import sys

numbers = re.compile('-?\d+(\.\d+)?')

with open(sys.argv[1]) as f:
    with open(sys.argv[2], 'w') as out:
        for line in f:
            first = next(numbers.finditer(line))
            if first:
                i = first.end()
                out.write(line[:i] + numbers.sub('<REDACTED>', line[i:]))
            else:
                out.write(line)
