#!/bin/bash

# Extract Release Names and Scene Groups from HDBits Browse Page
echo "🎯 Extracting Release Names from HDBits Browse"
echo "=============================================="

COOKIE="${HDBITS_SESSION_COOKIE:-PHPSESSID=ske6av9gkdp7n5usfgaiug7p6r; uid=1029013; pass=631fd2b772bba90112b10369eab5794719a6f4dcf07140b35aca32d484a27fa24989203c28cb8dcb52ebef5bf7cf63d176d81548efc2640f1c044e7587d8186d; cf_clearance=FQOnvz4X1iiAC47zrZul0dlhXblf5mC_pVpyH.5IRkM-1754176895-1.2.1.1-BwaSMNfIw6Ebt61bbGoDjgkt6UAWhkZTF9vQEYoXzoak7lkxWW8s1d..E9uQoRLITxpLSz0V1XguoPSa67Lex_ffkJNd8GSGZQPnuRGuMbRgiRGM3Lh6AhjV2f2UHT8NQz1LPJQaPR2RICaHESjbLTkW.ej1ybqhRnE.LzuDHxYlttdh7hg_PKwdLYuIINjdYvxE7Vmbo4UrS83aRnSud9Auz1A1LWpGY7qh2Xxf9mA; hush=e3c2a0b342a1ea913a8bc0b56e2deebcf807e40cde8a71d9676fc9dfdd74a059922a6a68975378ea89ddfd4de8bbac2b10a07865aa2088c675017e4a7fc8bc5f; hash=ebaa34a4efe6999a30cf0054db4f85bbff0718fcf46f4ce514fd488ee0ce74f247665e1d94af3fc3ae46557ac2507a413c0129893a4356c86eebf3d391f21528}"

# Get page content  
page_content=$(curl -s -H "Cookie: $COOKIE" "https://hdbits.org/browse.php?c1=1&o1=1&incldead=0")

echo "📥 Fetched $(echo "$page_content" | wc -c) characters"

# Extract release names from details.php links
echo "🎬 Extracting release names..."
release_names=$(echo "$page_content" | grep -oE 'href="/details\.php\?id=[0-9]+[^"]*"[^>]*title="[^"]*"[^>]*>([^<]+)</a>' | sed 's/.*>\([^<]*\)<\/a>/\1/' | head -10)

if [ -z "$release_names" ]; then
    # Alternative extraction method
    echo "🔄 Trying alternative extraction..."
    release_names=$(echo "$page_content" | grep -oE '<a[^>]*href="/details\.php[^"]*"[^>]*>([^<]+)</a>' | sed 's/.*>\([^<]*\)<\/a>/\1/' | grep -E '[0-9]{4}.*[0-9]+p' | head -10)
fi

if [ -n "$release_names" ]; then
    echo "✅ Found release names:"
    echo "$release_names"
    
    echo ""
    echo "🏷️  Extracting scene groups from release names:"
    echo "$release_names" | while read -r release; do
        if [[ $release =~ -([A-Za-z0-9]+)$ ]]; then
            echo "   $release → ${BASH_REMATCH[1]}"
        elif [[ $release =~ \[([A-Za-z0-9]+)\]$ ]]; then
            echo "   $release → ${BASH_REMATCH[1]}"
        else
            echo "   $release → No group detected"
        fi
    done
else
    echo "❌ No release names extracted"
    echo "🔍 Debugging HTML structure..."
    
    # Look for any patterns with movie/TV characteristics
    echo "$page_content" | grep -oE '[0-9]{4}.*[0-9]+p.*x26[45]' | head -5
    echo "$page_content" | grep -oE 'BluRay.*x26[45].*-[A-Z]+' | head -5
fi

echo ""
echo "✅ Release extraction test complete"