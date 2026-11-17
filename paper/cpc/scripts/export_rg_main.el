;;; export_rg_main.el --- Export the CPC manuscript -*- lexical-binding: t; -*-

(require 'org)
(require 'ox-latex)
(when (require 'ox-extra nil t)
  (ox-extras-activate '(ignore-headlines)))

(add-to-list 'org-latex-classes
             '("elsarticle"
               "\\documentclass[preprint,12pt]{elsarticle}"
               ("\\section{%s}" . "\\section*{%s}")
               ("\\subsection{%s}" . "\\subsection*{%s}")
               ("\\subsubsection{%s}" . "\\subsubsection*{%s}")
               ("\\paragraph{%s}" . "\\paragraph*{%s}")
               ("\\subparagraph{%s}" . "\\subparagraph*{%s}")))
(setq org-latex-packages-alist nil)
(setq org-latex-listings t)
(setq org-latex-src-block-backend 'listings)
(setq org-latex-prefer-user-labels t)
(setq org-export-time-stamp-file nil)
(setq org-latex-default-packages-alist
      '(("" "graphicx" t)
        ("" "rotating" nil)
        ("normalem" "ulem" t)))

(let* ((script-directory
        (file-name-directory (or load-file-name buffer-file-name)))
       (source
        (expand-file-name "../src/rg_main.org" script-directory))
       (output
        (expand-file-name "../src/rg_main.tex" script-directory)))
  (find-file source)
  (let ((org-confirm-babel-evaluate nil))
    (org-export-to-file 'latex output))
  (with-current-buffer (find-file-noselect output)
    (delete-trailing-whitespace)
    (save-buffer)
    (kill-buffer)))
