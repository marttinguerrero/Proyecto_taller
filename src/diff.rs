pub struct Diff;

/// en los diff guardo todo:
/// Add(linea)
/// Remove(linea)
/// Same(linea)
///
/// itero sobre los dos diffs simultaneamente
/// si ambos son Same escribo la linea y avanzo ambas pos
/// si uno es remove y el otro same no escribo nada y avanzo los dos
/// si uno es add y el otro same escribo el add y avanzo solo el add
/// si uno es add y el otro remove conflicto hasta que los dos sean same?
/// si ambos son add conflicto hasta que ambos sean same?

#[derive(PartialEq, Debug)]
pub enum ModificationType {
    Same(String),
    Add(String),
    Remove(String),
}

impl Diff {
    // Given two texts (one is a modified version of the first) it returns a vector
    // of ModificationType enum which represents what happened to that line in the change.
    // It can be either the same in both, removed from the original or added in the modified
    // version. The length of the vec is the length of the original plus the ammount of added lines.
    pub fn diff(original: &str, modified: &str) -> Vec<ModificationType> {
        let mut diff: Vec<ModificationType> = Vec::new();

        let original_lines: Vec<&str> = original.lines().collect();
        let modified_lines: Vec<&str> = modified.lines().collect();

        let lcs = Self::longest_common_line_subsequence(&original_lines, &modified_lines);

        let mut i = 0;
        let mut j = 0;

        for line in lcs {
            while i < original_lines.len() && original_lines[i] != line {
                diff.push(ModificationType::Remove(original_lines[i].to_string()));
                i += 1;
            }

            while j < modified_lines.len() && modified_lines[j] != line {
                diff.push(ModificationType::Add(modified_lines[j].to_string()));
                j += 1;
            }

            // Line is common to both texts
            diff.push(ModificationType::Same(line.clone()));

            i += 1;
            j += 1;
        }

        while i < original_lines.len() {
            diff.push(ModificationType::Remove(original_lines[i].to_string()));

            i += 1;
        }

        while j < modified_lines.len() {
            diff.push(ModificationType::Add(modified_lines[j].to_string()));
            j += 1;
        }

        diff
    }

    /// Given two vecs of &str representing the lines in two texts it returns a Vec of Strings
    /// which is the longest common subsequence of lines shared by both texts
    fn longest_common_line_subsequence(lines1: &[&str], lines2: &[&str]) -> Vec<String> {
        let len1 = lines1.len();
        let len2 = lines2.len();

        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

        for i in 1..=len1 {
            for j in 1..=len2 {
                if lines1[i - 1] == lines2[j - 1] {
                    matrix[i][j] = matrix[i - 1][j - 1] + 1;
                } else {
                    matrix[i][j] = matrix[i - 1][j].max(matrix[i][j - 1]);
                }
            }
        }

        let mut lcs = Vec::new();
        let mut i = len1;
        let mut j = len2;

        while i > 0 && j > 0 {
            if lines1[i - 1] == lines2[j - 1] {
                lcs.push(lines1[i - 1].to_string());
                i -= 1;
                j -= 1;
            } else if matrix[i - 1][j] > matrix[i][j - 1] {
                i -= 1;
            } else {
                j -= 1;
            }
        }

        lcs.reverse();
        lcs
    }
}

#[cfg(test)]
mod tests_diff {
    use crate::diff::Diff;

    #[test]
    fn test_longest_common_line_subsequence() {
        // Test case 1: Common subsequence exists
        let lines1 = vec!["line 1", "line 2", "line 3"];
        let lines2 = vec!["line 1", "line 4", "line 3"];
        assert_eq!(
            Diff::longest_common_line_subsequence(&lines1, &lines2),
            vec!["line 1", "line 3"]
        );

        // Test case 2: No common subsequence
        let lines1 = vec!["line 1", "line 2", "line 3"];
        let lines2 = vec!["line 4", "line 5", "line 6"];
        assert_eq!(
            Diff::longest_common_line_subsequence(&lines1, &lines2),
            Vec::<String>::new()
        );

        // Test case 3: Empty input
        let lines1: Vec<&str> = Vec::new();
        let lines2: Vec<&str> = Vec::new();
        assert_eq!(
            Diff::longest_common_line_subsequence(&lines1, &lines2),
            Vec::<String>::new()
        );

        // Test case 4: Single-line common subsequence
        let lines1 = vec!["line 1"];
        let lines2 = vec!["line 1"];
        assert_eq!(
            Diff::longest_common_line_subsequence(&lines1, &lines2),
            vec!["line 1"]
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_common_subsequence() {
        let original = "line 1\nline 2\nline 3";
        let modified = "line 1\nline 4\nline 3";
        let result = Diff::diff(original, modified);
        assert_eq!(
            result,
            vec![
                ModificationType::Same("line 1".to_string()),
                ModificationType::Remove("line 2".to_string()),
                ModificationType::Add("line 4".to_string()),
                ModificationType::Same("line 3".to_string())
            ]
        );
    }

    #[test]
    fn test_diff_no_common_subsequence() {
        let original = "line 1\nline 2\nline 3";
        let modified = "line 4\nline 5\nline 6";
        let result = Diff::diff(original, modified);
        assert_eq!(
            result,
            vec![
                ModificationType::Remove("line 1".to_string()),
                ModificationType::Remove("line 2".to_string()),
                ModificationType::Remove("line 3".to_string()),
                ModificationType::Add("line 4".to_string()),
                ModificationType::Add("line 5".to_string()),
                ModificationType::Add("line 6".to_string()),
            ]
        );
    }

    #[test]
    fn test_diff_empty_input() {
        let original = "";
        let modified = "";
        let result = Diff::diff(original, modified);
        assert_eq!(result, Vec::<ModificationType>::new());
    }

    #[test]
    fn test_diff_single_same_line_common_subsequence() {
        let original = "line 1";
        let modified = "line 1";
        let result = Diff::diff(original, modified);
        assert_eq!(result, vec![ModificationType::Same("line 1".to_string())]);
    }

    #[test]
    fn test_diff_one_line_common_subsequence() {
        let original = "line 1";
        let modified = "line0\nline 1";
        let result = Diff::diff(original, modified);
        assert_eq!(
            result,
            vec![
                ModificationType::Add("line0".to_string()),
                ModificationType::Same("line 1".to_string())
            ]
        );

        let modified = "line 1\nline 2";
        let result = Diff::diff(original, modified);
        assert_eq!(
            result,
            vec![
                ModificationType::Same("line 1".to_string()),
                ModificationType::Add("line 2".to_string())
            ]
        );
    }
}
