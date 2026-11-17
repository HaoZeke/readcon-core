;;; export_rg_main.el --- Export the CPC manuscript -*- lexical-binding: t; -*-

(let* ((script-directory
        (file-name-directory (or load-file-name buffer-file-name)))
       (source
        (expand-file-name "../src/rg_main.org" script-directory))
       (output
        (expand-file-name "../src/rg_main.tex" script-directory)))
  (with-temp-buffer
    (set-buffer-multibyte nil)
    (insert-file-contents-literally source)
    (write-region nil nil output nil 'silent)))
