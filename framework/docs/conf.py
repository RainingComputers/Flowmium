# Configuration file for the Sphinx documentation builder.
#
# For the full list of built-in configuration values, see the documentation:
# https://www.sphinx-doc.org/en/master/usage/configuration.html

# -- Project information -----------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#project-information

import sys
import os

project = "flowmium"
copyright = "2023, RainingComputers"
author = "RainingComputers"

# -- General configuration ---------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#general-configuration

extensions: list[str] = [
    "sphinx_rtd_theme",
    "sphinx.ext.napoleon",
    "sphinx_toolbox.decorators",
    "sphinx_markdown_builder",
]

templates_path = ["_templates"]
exclude_patterns = ["_build", "Thumbs.db", ".DS_Store"]

sys.path.insert(0, os.path.abspath(".."))


# -- Options for HTML output -------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#options-for-html-output

html_theme = "sphinx_rtd_theme"

html_static_path = ["_static"]
