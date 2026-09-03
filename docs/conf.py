# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
#
# Any modifications or derivative works of this code must retain this
# copyright notice, and modified files need to carry a notice indicating
# that they have been altered from the originals.

project = "Cqlib"
author = "Cqlib contributors"
copyright = "2026, Cqlib contributors"

from pygments.lexers.special import TextLexer
from sphinx.highlighting import lexers

extensions = [
    "myst_parser",
]

source_suffix = {
    ".rst": "restructuredtext",
    ".md": "markdown",
}

root_doc = "index"
language = "zh_CN"

exclude_patterns = [
    "_build",
    "Thumbs.db",
    ".DS_Store",
]

html_theme = "classic"
html_title = "Cqlib 文档"
html_show_sourcelink = False

myst_heading_anchors = 3

lexers["mermaid"] = TextLexer()
