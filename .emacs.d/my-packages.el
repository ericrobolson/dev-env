;; https://y.tsutsumi.io/2014/02/01/emacs-from-scratch-part-2-package-management/

;; Package archives
(require 'package)
(add-to-list 'package-archives '("melpa" . "https://melpa.org/packages/") t)
;; Comment/uncomment this line to enable MELPA Stable if desired.  See `package-archive-priorities`
;; and `package-pinned-packages`. Most users will not need or want to do this.
;;(add-to-list 'package-archives '("melpa-stable" . "https://stable.melpa.org/packages/") t)
(package-initialize)

;; Refresh the packages
(package-refresh-contents)
(package-install 'use-package)

;; Load the packages
(dolist 
    (package '(slime nim-mode solarized-theme))
    (unless 
        (package-installed-p package)
        (package-install package)))


