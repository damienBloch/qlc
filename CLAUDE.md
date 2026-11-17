# Project guidelines

- The code should be written to be easily reviewable and understandable by humans.
- Changes should be made in small, incremental steps, that can be individually reviewed and understood.
- Use simple code and avoid unnecessary complexity.
- Having a clean architecture is especially important. There should be an architecture document with diagrams.
- When writing prose, use simple technical English and avoid complex sentences, idioms, metaphors or punchlines.
- Compilation should be split in stages with an intermediate representation output by one stage and consumed by the next  
stage.
- It should be relatively easy to change the algorithm of a pass without having to change the rest of the compiler.  
This let us start with simple passes and then improve them later.
- Unit tests should be written when there are pure functions for which it is easy to test the behavior.
- Large tests that use a large portion of the compiler should be written as integration that tests that only test the  
interface of the compiler and not the internal implementation.
- It is important that tests are as readable and reviewable as the rest of the code. Don't hesitate to write utility 
functions to make tests more readable and avoid repeating boilerplate code.


# Code guidelines

- In the code, use "expect" with a reason for behavior that should not occur. Use "todo" for what is not decided yet.  
  It is crucial to panic rather than to silently ignore an unexpected situation, even if someone latter uses the function in a way that is not expected.
- `expect` should be bubbled up as high as possible, near the place where it is easy to check that the invariant is satisfied.
- Documentation should describe what the code does, not how.
- Documentation should be written in the smallest scope where it is relevant. Functions or types, then modules, then crates, then project architecture document.
- Items in a file should be sorted in logical, with public and most important items first, and then in breadth-first descending order. 

# Compiler guidelines

- As much check as possible should be done at compile time rather than at runtime.
- Functions should be checked individually. A function body that typechecks should not raise any error when called in  
a way that satisfies its signature.