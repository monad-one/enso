package org.enso.base.text;

import com.ibm.icu.text.BreakIterator;
import com.ibm.icu.text.CaseMap;
import com.ibm.icu.text.CaseMap.Fold;
import java.util.Locale;

/**
 * Represents a string transformed using Unicode Case Folding which can be used for case insensitive
 * comparisons.
 *
 * <p>It contains facilities for converting indices in the transformed string to corresponding
 * indices back in the original string.
 */
public class CaseFoldedString {
  private final String foldedString;

  /**
   * A mapping from code units in the transformed string to their corresponding graphemes in the
   * original string.
   *
   * <p>The mapping must be valid from indices from 0 to @{code foldedString.length()+1}
   * (inclusive).
   */
  private final int[] graphemeIndexMapping;

  /**
   * Constructs a new instance of the folded string.
   *
   * @param foldeString the string after applying the case folding transformation
   * @param graphemeIndexMapping a mapping created during the transformation which maps code units
   *     in the transformed string to their corresponding graphemes in the original string
   */
  private CaseFoldedString(String foldeString, int[] graphemeIndexMapping) {
    this.foldedString = foldeString;
    this.graphemeIndexMapping = graphemeIndexMapping;
  }

  /**
   * Maps a code unit in the folded string to the corresponding grapheme in the original string.
   *
   * @param codeunitIndex the index of the code unit in the folded string, valid indices range from
   *     0 to {@code getFoldedString().length()+1} (inclusive), allowing to also ask for the
   *     position of the end code unit which is located right after the end of the string - which
   *     should always map to the analogous end grapheme.
   * @return the index of the grapheme from the original string that after applying the
   *     transformation contains the requested code unit
   */
  public int codeUnitToGraphemeIndex(int codeunitIndex) {
    if (codeunitIndex < 0 || codeunitIndex > this.foldedString.length()) {
      throw new IndexOutOfBoundsException(codeunitIndex);
    }
    return graphemeIndexMapping[codeunitIndex];
  }

  /** Returns the transformed string. */
  public String getFoldedString() {
    return foldedString;
  }

  /**
   * Folds a string remembering the mapping from code units to its original grapheme cluster
   * indices.
   *
   * @param charSequence a sequence of UTF-16 characters to transform
   * @param locale the locale to use as a reference for case folding; it is needed because Turkish
   *     and Azerbaijani locales handle casing of the letter `i` in a different way than other
   *     locales
   * @return a {@code CaseFoldedString} instance which contains the transformed string and allows to
   *     map its code units to original grapheme clusters
   */
  public static CaseFoldedString fold(CharSequence charSequence, Locale locale) {
    BreakIterator breakIterator = BreakIterator.getCharacterInstance();
    breakIterator.setText(charSequence);
    StringBuilder stringBuilder = new StringBuilder(charSequence.length());
    Fold foldAlgorithm = caseFoldAlgorithmForLocale(locale);
    IntArrayBuilder index_mapping = new IntArrayBuilder(charSequence.length() + 1);

    // We rely on the fact that ICU Case Folding is _not_ context-sensitive, i.e. the mapping of
    // each grapheme cluster is independent of surrounding ones. Regular casing is
    // context-sensitive.
    int current = breakIterator.current();
    int next;
    int grapheme_index = 0;
    while ((next = breakIterator.next()) != BreakIterator.DONE) {
      CharSequence grapheme = new StringSlice(charSequence, current, next);
      String foldedGrapheme = foldAlgorithm.apply(grapheme);
      stringBuilder.append(foldedGrapheme);
      for (int i = 0; i < foldedGrapheme.length(); ++i) {
        index_mapping.add(grapheme_index);
      }

      grapheme_index++;
      current = next;
    }

    // The mapping should also be able to handle a {@code str.length()} query, so we add one more
    // element to the mapping pointing to a non-existent grapheme after the end of the text.
    index_mapping.add(grapheme_index);

    return new CaseFoldedString(
        stringBuilder.toString(), index_mapping.unsafeGetStorageAndInvalidateTheBuilder());
  }

  /**
   * A helper function which folds the string without remembering the index mapping.
   *
   * <p>It should be used when the index mapping is not needed, as its implementation is much more
   * efficient.
   *
   * @param charSequence a sequence of UTF-16 characters to transform
   * @param locale the locale to use as a reference for case folding; it is needed because Turkish
   *     and Azerbaijani locales handle casing of the letter `i` in a different way than the others
   * @return the folded string
   */
  public static String simpleFold(CharSequence string, Locale locale) {
    return caseFoldAlgorithmForLocale(locale).apply(string);
  }

  private static final Locale AZ_LOCALE = new Locale("az");
  private static final Locale TR_LOCALE = new Locale("tr");

  /**
   * Returns a case folding algorithm appropriate for the given locale.
   *
   * <p>The algorithm is locale-dependent because Turkish and Azerbaijani locales handle casing of
   * the letter `i` in a different way than other locales.
   */
  public static Fold caseFoldAlgorithmForLocale(Locale locale) {
    if (locale.equals(AZ_LOCALE) || locale.equals(TR_LOCALE)) {
      return CaseMap.fold().turkic();
    }
    return CaseMap.fold();
  }
}