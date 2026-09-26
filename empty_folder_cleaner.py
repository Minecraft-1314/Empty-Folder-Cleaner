"""Launch Empty Folder Cleaner from the package modules."""

import sys

try:
    from PyQt6.QtWidgets import QApplication, QStyleFactory
except ImportError:
    sys.exit("PyQt6 is required. Install it with: python -m pip install PyQt6")

from empty_folder_cleaner import MainWindow


def main():
    app = QApplication(sys.argv)
    fusion = QStyleFactory.create("Fusion")
    if fusion is not None:
        app.setStyle(fusion)
    window = MainWindow()
    exit_code = app.exec()
    window.deleteLater()
    return exit_code


if __name__ == "__main__":
    sys.exit(main())
