read_con_file_iterator reads through ConFrameReader.
It no longer reads the whole file into one string.
Invalid UTF-8 fails on the first read instead of on open.
The C struct is still two pointers, and the status codes stay the same.
