#include "exprtk.hpp"

// Disclaimer: I'm quite new with C++, not everything might be solved in
// the best manner. Suggestions welcome.


// helpers

template <typename T>
struct sized_array {
  size_t size;
  T *elements;
};

char *to_c_str(const std::string &s) {
  char *pc = new char[s.size() + 1];
  std::strcpy(pc, s.c_str());
  return pc;
}

sized_array<const char *> *strings_to_sized_array(
    const std::vector<std::string> v) {
  sized_array<const char *> *out = new sized_array<const char *>;
  out->size = v.size();
  out->elements = new const char *[out->size];

  for (size_t i = 0; i < out->size; i++) {
    out->elements[i] = to_c_str(v[i]);
  }

  return out;
}

struct parser_err {
  bool is_err;
  int mode;
  const char *token_type;
  const char *token_value;
  // const char* mode;
  const char *diagnostic;
  const char *error_line;
  size_t line_no;
  size_t column_no;
};

// adds new symbols to a vector

template <typename T>
struct symbol_resolver : public exprtk::parser<T>::unknown_symbol_resolver {
  typedef typename exprtk::parser<T>::unknown_symbol_resolver usr_t;
  std::vector<std::string> names;

  bool process(const std::string &unknown_symbol,
               typename usr_t::usr_symbol_type &st, T &default_value,
               std::string &error_message) {
    st = exprtk::parser<T>::unknown_symbol_resolver::e_usr_variable_type;
    default_value = T(0);
    error_message.clear();
    names.push_back(unknown_symbol);
    return true;
  }
};

// Functions (not sure if there is a way to avoid repetition)

template <typename T>
struct var1_func : public exprtk::ifunction<T> {
  T (*cb)(void *, T);
  void *user_data;
  var1_func(T (*c)(void *, T), void *d) : exprtk::ifunction<T>(1) {
    cb = c;
    user_data = d;
  }
  T operator()(const T &a) {
    return cb(user_data, a);
  }
};

template <typename T>
struct var2_func : public exprtk::ifunction<T> {
  T (*cb)(void *, T, T);
  void *user_data;
  var2_func(T (*c)(void *, T, T), void *d) : exprtk::ifunction<T>(2) {
    cb = c;
    user_data = d;
  }
  T operator()(const T &a, const T &b) {
    return cb(user_data, a, b);
  }
};

template <typename T>
struct var3_func : public exprtk::ifunction<T> {
  T (*cb)(void *, T, T, T);
  void *user_data;
  var3_func(T (*c)(void *, T, T, T), void *d) : exprtk::ifunction<T>(3) {
    cb = c;
    user_data = d;
  }
  T operator()(const T &a, const T &b, const T &c) {
    return cb(user_data, a, b, c);
  }
};

template <typename T>
struct var4_func : public exprtk::ifunction<T> {
  T (*cb)(void *, T, T, T, T);
  void *user_data;
  var4_func(T (*c)(void *, T, T, T, T), void *d) : exprtk::ifunction<T>(4) {
    cb = c;
    user_data = d;
  }
  T operator()(const T &a, const T &b, const T &c, const T &d) {
    return cb(user_data, a, b, c, d);
  }
};


// these methods don't depend on a specific precision

extern "C" {
    typedef exprtk::parser<double> Parser;
    typedef symbol_resolver<double> UnknownSymbolResolver;
    typedef exprtk::symbol_table<double> SymbolTable;
    typedef exprtk::expression<double> Expression;

    // Parser

    Parser *parser_new() { return new Parser; }

    void parser_destroy(Parser *p) { delete p; }

    bool parser_compile(Parser *p, const char *s, Expression *e) {
      return p->compile((const std::string &)s, *e);
    }

    struct compile_result {
      bool ok;
      sized_array<const char *> *vars;
    };

    compile_result parser_compile_resolve(Parser *p, const char *s, Expression *e) {
      UnknownSymbolResolver resolver;
      compile_result result;

      p->enable_unknown_symbol_resolver(&resolver);

      result.ok = p->compile((const std::string &)s, *e);

      exprtk::symbol_table<double> t = e->get_symbol_table(0);

      result.vars = strings_to_sized_array(resolver.names);

      p->disable_unknown_symbol_resolver();

      return result;
    }

    // void var_pointer_list_free(var_pointer_list *l) { delete l; }

    parser_err *parser_error(Parser *p) {
      // TODO: it seems p->get_error(0) creates a copy of the error (why?)
      // therefore we have to heap allocate the output
      parser_err *out = new parser_err;
      if (p->error_count() > 0) {
        out->is_err = true;
        exprtk::parser_error::type err = p->get_error(0);
        out->mode = err.mode;
        out->token_type = to_c_str(exprtk::lexer::token::to_str(err.token.type));
        out->token_value = to_c_str(err.token.value);
        out->diagnostic = to_c_str(err.diagnostic);
        out->error_line = to_c_str(err.error_line);
        out->line_no = err.line_no;
        out->column_no = err.column_no;
      }
      return out;
    }

    void parser_error_free(parser_err *e) {
      delete[] e->token_type;
      delete[] e->token_value;
      delete[] e->diagnostic;
      delete[] e->error_line;
      delete e;
    }

    // String values: Rust cannot deal with C++ strings by itself

    std::string *cpp_string_create(const char *s, size_t len) {
      return new std::string(s, len);
    }

    void cpp_string_set(std::string *s, const char *replacement, size_t len) {
      s->assign(replacement, len);
    }

    const char *cpp_string_get(const std::string *s) { return s->c_str(); }

    void cpp_string_free(std::string *s) { delete s; }

    // Symbol table

    SymbolTable *symbol_table_new() { return new SymbolTable; }

    void symbol_table_destroy(SymbolTable *t) { delete t; }

    bool symbol_table_add_variable(SymbolTable *t, char *variable_name,
                                   double *value, bool is_constant = false) {
      return t->add_variable(std::string(variable_name), *value, is_constant);
    }

    bool symbol_table_create_variable(SymbolTable *t, char *variable_name,
                                      const double value) {
      return t->create_variable(std::string(variable_name), value);
    }

    bool symbol_table_add_constant(SymbolTable *t, char *variable_name,
                                   const double value) {
      return t->add_constant(std::string(variable_name), value);
    }

    bool symbol_table_add_stringvar(SymbolTable *t, char *variable_name,
                                    std::string *string, bool is_const) {
      return t->add_stringvar(std::string(variable_name), *string, is_const);
    }

    bool symbol_table_create_stringvar(SymbolTable *t, char *variable_name,
                                       char *string) {
      return t->create_stringvar(std::string(variable_name), std::string(string));
    }

    bool symbol_table_add_vector(SymbolTable *t, char *name, double *vec,
                                 const size_t len) {
      return t->add_vector(std::string(name), vec, len);
    }

    bool symbol_table_remove_variable(SymbolTable *t, char *name) {
      return t->remove_variable(std::string(name), true);
    }

    bool symbol_table_remove_stringvar(SymbolTable *t, char *name) {
      return t->remove_stringvar(std::string(name));
    }

    bool symbol_table_remove_vector(SymbolTable *t, char *name) {
      return t->remove_vector(std::string(name));
    }

    void symbol_table_clear_variables(SymbolTable *t) { t->clear_variables(true); }

    void symbol_table_clear_strings(SymbolTable *t) { t->clear_strings(); }

    void symbol_table_clear_vectors(SymbolTable *t) { t->clear_vectors(); }

    double *symbol_table_variable_ref(SymbolTable *t, char *variable_name) {
      return &t->variable_ref(std::string(variable_name));
    }

    std::string *symbol_table_stringvar_ref(SymbolTable *t, char *variable_name) {
      return &t->stringvar_ref(std::string(variable_name));
    }

    const double *symbol_table_vector_ptr(SymbolTable *t, char *variable_name) {
      return (double *) t->get_vector(std::string(variable_name))->data();
    }

    size_t symbol_table_variable_count(SymbolTable *t) {
      return t->variable_count();
    }

    size_t symbol_table_stringvar_count(SymbolTable *t) {
      return t->stringvar_count();
    }

    size_t symbol_table_vector_count(SymbolTable *t) { return t->vector_count(); }

    bool symbol_table_add_constants(SymbolTable *t) { return t->add_constants(); }

    bool symbol_table_add_pi(SymbolTable *t) { return t->add_pi(); }

    bool symbol_table_add_epsilon(SymbolTable *t) { return t->add_epsilon(); }

    bool symbol_table_add_infinity(SymbolTable *t) { return t->add_infinity(); }

    bool symbol_table_is_constant_node(SymbolTable *t, char *name) {
      return t->is_constant_node(std::string(name));
    }

    bool symbol_table_is_constant_string(SymbolTable *t, char *name) {
      return t->is_constant_string(std::string(name));
    }

    sized_array<const char *> *symbol_table_get_variable_list(SymbolTable *t) {
      std::vector<std::string> vlist;
      t->get_variable_list(vlist);
      return strings_to_sized_array(vlist);
    }

    sized_array<const char *> *symbol_table_get_stringvar_list(SymbolTable *t) {
      std::vector<std::string> slist;
      t->get_stringvar_list(slist);
      return strings_to_sized_array(slist);
    }

    sized_array<const char *> *symbol_table_get_vector_list(SymbolTable *t) {
      std::vector<std::string> vlist;
      t->get_vector_list(vlist);
      return strings_to_sized_array(vlist);
    }

    void string_array_free(sized_array<char *> *c) {
      char **strings = c->elements;
      int n = c->size;
      for (int i = 0; i < n; i++) {
        delete[] strings[i];
      }
      delete[] strings;
      delete c;
    }

    bool symbol_table_symbol_exists(SymbolTable *t, char *variable_name) {
      return t->symbol_exists(std::string(variable_name));
    }

    bool symbol_table_valid(SymbolTable *t) { return t->valid(); }

    void symbol_table_load_from(SymbolTable *t, const SymbolTable *other) {
      t->load_from(*other);
    }

    // functions

    struct func_result {
      bool res;
      void *fn_pointer;
    };

    #define FUNC_DEF(ADD_NAME, FREE_NAME, STRUCT, ARGS...)         \
      func_result ADD_NAME(SymbolTable *t, char *name,             \
                     double (*cb)(void *, ARGS), void *user_data)  \
      {                                                            \
        STRUCT<double> *f = new STRUCT<double>(cb, user_data);     \
        func_result out;                                           \
        out.res = t->add_function(std::string(name), *f);          \
        out.fn_pointer = (void *)f;                                \
        return out;                                                \
      }                                                            \
                                                                   \
      void FREE_NAME(var1_func<double> *f) {                       \
        delete f;                                                  \
      }

    FUNC_DEF(symbol_table_add_func1, symbol_table_free_func1, var1_func,
             double);
    FUNC_DEF(symbol_table_add_func2, symbol_table_free_func2, var2_func,
             double, double);
    FUNC_DEF(symbol_table_add_func3, symbol_table_free_func3, var3_func,
             double, double, double);
    FUNC_DEF(symbol_table_add_func4, symbol_table_free_func4, var4_func,
             double, double, double, double);


    // Expression

    Expression *expression_new() { return new Expression; }

    void expression_destroy(Expression *e) { delete e; }

    void expression_register_symbol_table(Expression *e, SymbolTable *t) {
      e->register_symbol_table(*t);
    }

    double expression_value(Expression *e) { return e->value(); }
}
