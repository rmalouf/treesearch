import treesearch

query = """
N1 [pos="NOUN"];
Of [form="of"];
N2 [pos="NOUN"];
N1 -> Of;
Of -> N2;
"""

path = "/Volumes/Corpora/COHA/conll/*.conllu.gz"
pattern = treesearch.compile_query(query)

count = 0
for tree, match in treesearch.search_files(path, pattern):
    count += 1

print(count)
