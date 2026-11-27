import pytest

from dongne_gis.manifest import build_manifest


def test_default_attribution_included():
    m = build_manifest("t", 37.5, 127.0, 1500, 1500, 0, 120)
    assert m["format_version"] == 0
    assert any("OpenStreetMap" in a for a in m["attribution"])
    assert any("COPERNICUS" in a for a in m["attribution"])


def test_empty_attribution_rejected():
    with pytest.raises(ValueError):
        build_manifest("t", 0, 0, 1, 1, 0, 1, attribution=[" "])
