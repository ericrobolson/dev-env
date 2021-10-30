;; Note: to run with this file, execute:
;; `emacs -nw -q -l ./.emacs`

;; A pointer to a local file.

;; Load all specified files under `.emacs.d/`

(defun files 
    ()
'
    ("test.el" "my-packages.el"))

(defun format-file 
    (f)
    (concat default-directory ".emacs.d/" f))

(dolist 
    (f 
        (files))
    (load 
        (format-file f)))


;; Set SBCL to run.
(setq inferior-lisp-program "sbcl")

;;;;;;;;;;;;;;;;;;
;; Config stuff ;;
;;;;;;;;;;;;;;;;;;

;; theme
(load-theme 'solarized-wombat-dark t)

;; a visible bell
(setq visible-bell t)
(setq ring-bell-function 'ignore)

;; save autosave files in here
(setq backup-directory-alist
      `((".*" . ,temporary-file-directory)))
(setq auto-save-file-name-transforms
      `((".*" ,temporary-file-directory t)))

;; Set desktop mode
(desktop-save-mode 1)

;; Set line numbers to be visible
(global-display-line-numbers-mode)
