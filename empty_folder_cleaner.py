"""Launch Empty Folder Cleaner from the package modules."""

from PyQt6.QtWidgets import QApplication, QStyleFactory

from empty_folder_cleaner import MainWindow


def main():
    app = QApplication([])
    app.setStyle(QStyleFactory.create("Fusion"))
    window = MainWindow()
    app.exec()


if __name__ == "__main__":
    main()
