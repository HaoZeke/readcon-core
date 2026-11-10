read_extxyz returns atomic numbers as a fourth array.
A file with no species column raises instead of omitting that array.
Callers that unpack three values must unpack four.
