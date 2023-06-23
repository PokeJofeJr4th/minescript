mod lexer;

mod parser;

mod interpreter;

mod types {
    use crate::types::farey_approximation;

    #[test]
    fn rational_approximator() {
        assert_eq!(farey_approximation(0.5, 10), (1, 2));
        assert_eq!(farey_approximation(2.5, 10), (5, 2));
        assert_eq!(farey_approximation(2.0, 10), (2, 1));
        assert_eq!(farey_approximation(1.618, 10), (13, 8));
        assert_eq!(farey_approximation(1.618, 100), (144, 89));
        // examples from https://www.johndcook.com/blog/2010/10/20/best-rational-approximation/
        assert_eq!(farey_approximation(0.367_879, 10), (3, 8));
        assert_eq!(farey_approximation(0.367_879, 100), (32, 87));
    }
}
