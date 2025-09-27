"""
Private Job Scraper & Filter

A robust, private job monitoring service that runs on your own machine.
"""

import os
from pathlib import Path

# Version management
__version__ = "1.0.0"

def get_version():
    """Get the current version from VERSION file or fallback to __version__."""
    try:
        version_file = Path(__file__).parent / "VERSION"
        if version_file.exists():
            return version_file.read_text().strip()
    except Exception:
        pass
    return __version__

__version__ = get_version()