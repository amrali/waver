//! Error handling and error type tests.

use super::*;

#[test]
fn test_error_handling() {
    // Test error conditions and display/equality
    let wf = Waveform::<f32>::new(44100.0);

    // Test time_spectrum on generative waveform
    let err = time_spectrum(&wf, 1024, 512).unwrap_err();
    assert_eq!(err, Error::UnsupportedSource);

    // Test synthesize on generative waveform
    let err = synthesize(&wf, 1024, 512, 1).unwrap_err();
    assert_eq!(err, Error::UnsupportedSource);

    // Test error display and equality
    let error1 = Error::UnsupportedSource;
    let error2 = Error::UnsupportedSource;
    assert_eq!(error1, error2);
    assert_eq!(error1.clone(), error2);

    let error_string = format!("{:?}", error1);
    assert!(error_string.contains("UnsupportedSource"));
}

#[test]
fn test_error_display() {
    let error = Error::UnsupportedSource;
    let error_string = format!("{:?}", error);
    assert!(error_string.contains("UnsupportedSource"));
}

#[test]
fn test_error_equality() {
    let error1 = Error::UnsupportedSource;
    let error2 = Error::UnsupportedSource;
    assert_eq!(error1, error2);
    assert_eq!(error1.clone(), error2);
}
